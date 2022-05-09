use crate::package::DependencySource;
use std::collections::HashMap;

use crate::error::PckpError;
use lazy_static::lazy_static;
use reqwest;

//TODO: replace with real repo
pub const MAIN_REPO: &str = "https://raw.githubusercontent.com/camila314/ttest/master/index.txt";

lazy_static! {
    static ref REPO_CACHE: HashMap<String, String> = {
        reqwest::blocking::get(MAIN_REPO)
            .unwrap()
            .text()
            .unwrap()
            .replace(' ', "")
            .split('\n')
            .map(|x| x.split('|'))
            .map(|mut x| (x.next().unwrap().to_string(), x.next().unwrap().to_string()))
            .collect::<HashMap<_, _>>()
    };
}

fn find_in_repo(name: &str) -> Option<&String> {
    REPO_CACHE.get(name)
}

impl DependencySource {
    pub fn to_string(&self, parent_name: String) -> Result<String, PckpError> {
        match self {
            DependencySource::Url(a) => Ok(a.to_string()),
            DependencySource::Name(b) => match b.split('/').count().cmp(&2) {
                std::cmp::Ordering::Greater => {
                    return Err(PckpError::custom(
                        format!("Invalid dependency name '{}'", b),
                        Some(parent_name),
                    ));
                }
                std::cmp::Ordering::Equal => Ok("https://github.com/".to_string() + b),
                _ => match find_in_repo(b) {
                    Some(x) => Ok(x.to_string()),
                    None => Err(PckpError::custom(
                        format!("Unable to locate dependency '{}' in pckp repo", b),
                        Some(parent_name),
                    )),
                },
            },
        }
    }
}
