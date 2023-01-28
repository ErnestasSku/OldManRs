use std::path::Path;
use std::sync::Arc;

use serenity::framework::standard::macros::command;
use serenity::framework::standard::{CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use tokio::sync::RwLock;

use crate::story::story_structs::{StoryContainer, StoryBlock};
use crate::story::story_builder::map_stories_p;


#[derive(Debug, Clone)]
pub struct StoryListener {
    current_story_path: Option<Arc<StoryBlock>>
}

impl StoryListener {
    pub fn new(story: &Arc<StoryBlock>) -> StoryListener {
        StoryListener { current_story_path: Some(story.clone()) }
    }
}

pub struct StoryListenerContainer;
pub struct LoadedStoryContainer;

impl TypeMapKey for StoryListenerContainer {
    type Value = Arc<RwLock<std::collections::HashMap<UserId, StoryListener>>>;
}

impl TypeMapKey for LoadedStoryContainer {
    type Value = Arc<RwLock<Option<Arc<StoryBlock>>>>;
}



#[command]
#[aliases("start", "begin")]
#[description = "Lets you start a story which was selected"]
async fn start_story(ctx: &Context, msg: &Message) -> CommandResult {

    let (user_lock, story_lock) = {
        let data_read = ctx.data.read().await;

        (data_read.get::<StoryListenerContainer>().expect("Expected StoryListenerContainer in TypeMap").clone(),
        data_read.get::<LoadedStoryContainer>().expect("Expected LoadedStoryContainer in TypeMap").clone())
    };

    {
        let story = story_lock.read().await;
        let story_block = story.as_ref().cloned();

        match story_block {
            None => {
                msg.reply(ctx, "No story found").await?;
            },
            Some(story) => {
                let mut user_map = user_lock.write().await;
                
                let new_story = StoryListener::new(&story);

                let content = new_story.clone().current_story_path.map(|x| x.present()).unwrap();
                msg.reply(ctx, content).await?;
                
                user_map.insert(msg.author.id, new_story);
                
            }
        }

    }

    Ok(())
}

#[command]
#[aliases("action", "do")]
#[description = "You can make a choice"]
async fn action(ctx: &Context, msg: &Message) -> CommandResult {

    let command_name = msg.content.to_owned().split(' ').collect::<Vec<&str>>().get(1).map(|x| x.to_string());

    println!("Action - {:?}", &command_name);

    match command_name {
        None => {
            msg.reply(ctx, "invalid command").await?;
        },
        Some(command) => {
            let user_lock = {
                let data_read = ctx.data.read().await;
                data_read.get::<StoryListenerContainer>().expect("Expected StoryListenerContainer in TypeMap").clone()
                
            };

            {
                let mut user_map = user_lock.write().await;
                let user = user_map.get(&msg.author.id);
                let mut new_user_value = None;
                match user {
                    None => {
                        msg.reply(ctx, "User hasn't started the story yet").await?;

                    },
                    Some(user_prime) => {
                        println!("Inside user_prime block");
                        let mut index = -1;
                            match &user_prime.current_story_path {
                                None => {
                                    msg.reply(ctx, "No active story").await?;
                                },
                                Some(val) => {
                                    for (i, data) in val.path.lock().unwrap().iter().enumerate() {
                                        if data.1 == command {
                                            index = i as i32;
                                        }
                                        
                                        println!("\n\n{:?}\n\n", data);
                                    }
                                    
                                    println!("Index - {index}");
                                    if index == -1 {
                                        new_user_value = None;
                                    } else {
                                        new_user_value = Some(StoryListener::new(&val.path.lock().unwrap().get(index as usize).unwrap().0));
                                    }
                                } 
                            }
                    }
                }
                
                if new_user_value.is_some() {
                    let temp = new_user_value.clone();
                    user_map.insert(msg.author.id, new_user_value.unwrap());
                    println!("{:?}", &temp);
                    if let Some(st) = temp {
                        if let Some(message) = &st.current_story_path {
                            
                            msg.reply(ctx, message.present()).await?;
                        }
                    } else {
                        println!("It was None");
                    }
                }
            }
        }
    }

    

    Ok(())
}

#[command]
#[owners_only]
#[description = "Loads a story file from computer into memory. Usage: ~story load C:\\User\\...\\story_name.story"]
async fn load(ctx: &Context, msg: &Message) -> CommandResult {

    //TODO: refactor to use arg
    let file_path = msg.content.to_owned().split(' ').collect::<Vec<&str>>().get(1).map(|x| x.to_string());
    println!("{:?}", &file_path);
    if file_path.is_none() {
        msg.reply(ctx, "Error occurred during parsing of the command. (File path not supplied?)").await?;
        return Ok(());
    }
    let file_path = file_path.unwrap();
    let story = map_stories_p(&file_path);
    let file_name = Path::new(&file_path);
    let file_name = file_name.file_stem().unwrap().to_str().unwrap().to_string();

    match story {
        Ok(story) => {
            let story_lock = {
                let data_read = ctx.data.read().await;
                data_read.get::<StoryContainer>().expect("Expected StoryContainer in TypeMap").clone()
            };
            
            {
                let mut map = story_lock.write().await;
                map.insert(file_name, story);
            }
        },
        Err(err) => {
            msg.reply(ctx, err).await?;
            return Ok(());
        }
    }
    msg.reply(ctx, "Loaded successfully").await?;
    Ok(())
}

#[command]
#[owners_only]
#[description = "Prints a list of loaded stories."]
async fn read_loaded(ctx: &Context, msg: &Message) -> CommandResult {

    let story_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<StoryContainer>().expect("Expected StoryContainer in TypeMap").clone()
    };
    {
        let stories = story_lock.read().await.clone();
        
        println!("{:?}", &stories);
        let message = stories.into_keys()
            .collect::<Vec<String>>()
            .iter()
            .enumerate()
            .map(|(i, x)| i.to_string() + ". " +  x + "\n")
            .collect::<Vec<String>>()
            .concat();

        msg.reply(ctx, message).await?;
    }

    Ok(())
}

#[command]
#[owners_only]
#[description = "Selects a story to be played. Usage: ~story set_story storyName"]
async fn set_story(ctx: &Context, msg: &Message) -> CommandResult {

     //TODO: refactor this to use args struct
    let story_name = msg.content.to_owned().split(' ').collect::<Vec<&str>>().get(1).map(|x| x.to_string());
    if story_name.is_none() {
        msg.reply(ctx, "Error occurred during parsing of the command. (File path not supplied?)").await?;
        return Ok(());
    }
    let story_name = story_name.unwrap();
    
    let (curr_story_lock, story_lock) = {
        let data_read = ctx.data.read().await;

        (data_read.get::<LoadedStoryContainer>().expect("Expected LoadedStoryContainer in TypeMap").clone(),
         data_read.get::<StoryContainer>().expect("Expected  in TypeMap").clone())
    };

    {
        let story_map = story_lock.read().await;
        let story = story_map.get(&story_name);

        match story {
            None => {
                msg.reply(ctx, "There is no such story").await?;
            },
            Some(story) => {
                let mut current_story = curr_story_lock.write().await;
                current_story.replace(story.clone());
            
                msg.reply(ctx, "Loaded the story").await?;
            }
        }
    }

    Ok(())
}