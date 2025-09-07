#[derive(Debug)]
pub struct User {
    pub name: String,
    pub spotify_user_id: String,
    pub discord_user_id: String,
    pub order: usize,
}

pub struct UserMaster {
    pub users: Vec<User>,
}

impl UserMaster {
    pub fn get_next_user_by_spotify_id(&self, spotify_user_id: &str) -> Option<&User> {
        for (i, user) in self.users.iter().enumerate() {
            if user.spotify_user_id == spotify_user_id {
                return self.users.get(i + 1);
            }
        }
        None
    }
}
