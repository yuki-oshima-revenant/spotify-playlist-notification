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
                return self.users.get((i + 1) % self.users.len());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_next_user_by_spotify_id() {
        let user1 = User {
            name: "User1".to_string(),
            spotify_user_id: "spotify1".to_string(),
            discord_user_id: "discord1".to_string(),
            order: 1,
        };
        let user2 = User {
            name: "User2".to_string(),
            spotify_user_id: "spotify2".to_string(),
            discord_user_id: "discord2".to_string(),
            order: 2,
        };
        let user3 = User {
            name: "User3".to_string(),
            spotify_user_id: "spotify3".to_string(),
            discord_user_id: "discord3".to_string(),
            order: 3,
        };
        let user_master = UserMaster {
            users: vec![user1, user2, user3],
        };
        assert_eq!(
            user_master
                .get_next_user_by_spotify_id("spotify1")
                .unwrap()
                .spotify_user_id,
            "spotify2"
        );
        assert_eq!(
            user_master
                .get_next_user_by_spotify_id("spotify2")
                .unwrap()
                .spotify_user_id,
            "spotify3"
        );
        assert_eq!(
            user_master
                .get_next_user_by_spotify_id("spotify3")
                .unwrap()
                .spotify_user_id,
            "spotify1"
        );
        assert!(user_master.get_next_user_by_spotify_id("unknown").is_none());
    }
}
