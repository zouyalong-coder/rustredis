pub enum CommandGroup {
    CommandGroupGeneric,
    CommandGroupString,
    CommandGroupList,
    COMMAND_GROUP_SET,
    COMMAND_GROUP_SORTED_SET,
    COMMAND_GROUP_HASH,
    COMMAND_GROUP_PUBSUB,
    COMMAND_GROUP_TRANSACTIONS,
    COMMAND_GROUP_CONNECTION,
    COMMAND_GROUP_SERVER,
    COMMAND_GROUP_SCRIPTING,
    COMMAND_GROUP_HYPERLOGLOG,
    COMMAND_GROUP_CLUSTER,
    COMMAND_GROUP_SENTINEL,
    COMMAND_GROUP_GEO,
    COMMAND_GROUP_STREAM,
    COMMAND_GROUP_BITMAP,
    COMMAND_GROUP_MODULE,
}

pub struct CommandMeta {
    /// 
    declared_name: Vec<u8>, // name of the command.
    /// description of this command.
    summary: String, 
    /// 
    complexity: String,
    since: String,
    replace_by: Option<Vec<u8>>,
    deprecated_since: Option<String>,
    group: CommandGroup,
    /// tips for clients/proxies regarding this command
    tips: Vec<String>,
    flags: u64,
    acl_category: u64,
}


pub trait Command {
    
}