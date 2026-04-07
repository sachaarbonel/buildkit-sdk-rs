use buildkit_rs_proto::pb;

/// Ownership option for file operations.
#[derive(Debug, Clone)]
pub struct ChownOpt {
    pub user: Option<UserOpt>,
    pub group: Option<UserOpt>,
}

impl ChownOpt {
    /// Create a new ChownOpt with user and group specified by ID.
    pub fn new(uid: u32, gid: u32) -> Self {
        Self {
            user: Some(UserOpt::ById(uid)),
            group: Some(UserOpt::ById(gid)),
        }
    }

    /// Create a new ChownOpt with only a user ID.
    pub fn user_id(uid: u32) -> Self {
        Self {
            user: Some(UserOpt::ById(uid)),
            group: None,
        }
    }

    /// Create a new ChownOpt with user and group specified by name.
    pub fn by_name(user: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            user: Some(UserOpt::ByName(user.into())),
            group: Some(UserOpt::ByName(group.into())),
        }
    }

    pub(crate) fn to_pb(&self) -> pb::ChownOpt {
        pb::ChownOpt {
            user: self.user.as_ref().map(|u| u.to_pb()),
            group: self.group.as_ref().map(|g| g.to_pb()),
        }
    }
}

/// User identification for ownership.
#[derive(Debug, Clone)]
pub enum UserOpt {
    ById(u32),
    ByName(String),
}

impl UserOpt {
    pub(crate) fn to_pb(&self) -> pb::UserOpt {
        pb::UserOpt {
            user: Some(match self {
                UserOpt::ById(id) => pb::user_opt::User::ById(*id),
                UserOpt::ByName(name) => pb::user_opt::User::ByName(pb::NamedUserOpt {
                    name: name.clone(),
                    input: -1,
                }),
            }),
        }
    }
}
