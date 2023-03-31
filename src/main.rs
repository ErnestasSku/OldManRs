mod commands;
mod story;
mod utilities;

use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::Arc;

use serenity::async_trait;
use serenity::framework::standard::macros::{group, help};
use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandResult, HelpOptions,
};
use serenity::framework::*;
use serenity::http::Http;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::model::prelude::{Message, UserId};
use serenity::prelude::*;
use story::story_structs::StoryContainer;
use tracing::{error, info};
use update_informer::{registry, Check};

use crate::commands::general::*;
use crate::commands::math::*;
use crate::commands::owner::*;
use crate::story::story::*;

pub struct ShardManagerContainer;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(info, action, multiply, quit)]
struct General;

#[group]
#[commands(start_story, action, load, read_loaded, set_story, clear_story)]
#[prefixes("story", "s")]
#[description = "Commands related to the stories"]
#[default_command(action)]
struct Story;

#[tokio::main]
async fn main() {
    run_informer().await;
    run_bot().await;
}

async fn run_bot() {
    dotenv::dotenv().expect("Failed to load .env file");
    let token = env::var("TOKEN").expect("Expected a token in the environment");
    let http = Http::new(&token);

    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("~"))
        .help(&HELP)
        .group(&GENERAL_GROUP)
        .group(&STORY_GROUP);

    let intents = GatewayIntents::all();
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    setup_data(&client).await;

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    info!("Bot is starting...");
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

async fn run_informer() {
    let informer = update_informer::new(
        registry::GitHub,
        "https://github.com/ErnestasSku/Mnemosyne",
        "0.1.0",
    );

    if let Some(version) = informer.check_version().ok().flatten() {
        println!("New version is available: {}. Go to https://github.com/ErnestasSku/Mnemosyne to update", version);
    }
}

async fn setup_data(client: &Client) {
    let mut data = client.data.write().await;
    data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    data.insert::<StoryContainer>(Arc::new(RwLock::new(HashMap::default())));
    data.insert::<StoryListenerContainer>(Arc::new(RwLock::new(HashMap::default())));
    data.insert::<LoadedStoryContainer>(Arc::new(RwLock::new(None)));
}

#[help]
#[command_not_found_text = "Could not find: `{}`."]
#[max_levenshtein_distance(3)]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}
