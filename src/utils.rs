
pub fn author_name_from_msg(msg: &serenity::all::Message) -> String {
    let mut author_name: String = msg
        .author
        .global_name
        .clone()
        .unwrap_or(msg.author.name.clone());

    if let Some(member) = &msg.member {
        if let Some(nick) = &member.nick {
            author_name = nick.to_string();
        }
    }

    if author_name == "Purple Puppy" && msg.author.name != "purplepuppy" {
        return "Fake Deformed Purple Puppy".to_string();
    }
    author_name
}
