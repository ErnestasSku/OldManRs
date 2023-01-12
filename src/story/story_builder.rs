use std::{sync::Arc, fs};

use crate::story::{story_parser::*, story2::StoryBlock2};


pub fn map_stories_p(file: &String) -> Result<Arc<StoryBlock2>, String> {

    let file = fs::read_to_string(file);
    if let Err(error) = file {
        return  Err("Error happened during file read: ".to_string() + &error.to_string());
    }

    let file_s = &file.unwrap()[..];

    let parsed = story(file_s);
    let (_, parsed) = parsed.unwrap();

    let mut story_blocks: Vec<Arc<StoryBlock2>> = parsed.
        iter()
        .map(StoryBlock2::from_parse)
        .map(Arc::new)
        .collect();


    for i in 0..story_blocks.len() {
        let copied: Vec<Arc<StoryBlock2>> = story_blocks.to_vec();
        let current = &mut story_blocks[i];
        
        let mut to_map: Vec<(String, String, String)> = Vec::new();
        let mut paths: Vec<(Arc<StoryBlock2>, String, String)> = Vec::new();
        
        //Find paths from parse
        for p in parsed.iter() {
            if p.id == current.id {
                for p_child in p.children.iter() {
                    to_map.push((
                        p_child.next_path.clone(),
                        p_child.command.clone(),
                        p_child.label.clone()
                    ));

                }
            }
        }


        //Find blocks   
        for item in copied.iter() {
            let elem = to_map.iter().find(|x| x.0 == item.id);

            if let Some(el) = elem {
                paths.push((
                    item.clone(),
                    el.1.clone(),
                    el.2.clone()
                ));
            }
        }
        *current.path.lock().unwrap() = paths;
        
    }

    let mut head = Err("No story was created".to_string());
    for x in story_blocks.iter() {
        if x.id == "START" {
            head = Ok(x.clone());
        }
    }
    head
}
