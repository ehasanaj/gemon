use crate::project::Project;

pub struct Effector;

impl Effector {
    pub fn apply_env_to_args(args: Vec<String>) -> Vec<String> {
        match Project::env() {
            Some(env) => {
                let mut args = args;
                for (key, value) in env.values() {
                    let formated_key = format!("{{{}}}", key);
                    args = args
                        .into_iter()
                        .map(|arg| arg.replace(&formated_key, &value))
                        .collect();
                }
                args
            }
            None => args,
        }
    }

    pub fn apply_env_to_string(text: String) -> String {
        match Project::env() {
            Some(env) => {
                let mut text = text;
                for (key, value) in env.values() {
                    let formated_key = format!("{{{}}}", key);
                    text = text.replace(&formated_key, &value);
                }
                text
            }
            None => text,
        }
    }
}
