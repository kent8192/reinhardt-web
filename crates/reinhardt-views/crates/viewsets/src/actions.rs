/// Action type for ViewSet operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    List,
    Retrieve,
    Create,
    Update,
    PartialUpdate,
    Destroy,
    Custom(&'static str),
}

/// Action metadata
pub struct Action {
    pub action_type: ActionType,
    pub detail: bool, // Whether this action operates on a single object
}

impl Action {
    /// Documentation for `list`
    ///
    pub fn list() -> Self {
        Self {
            action_type: ActionType::List,
            detail: false,
        }
    }
    /// Documentation for `retrieve`
    ///
    pub fn retrieve() -> Self {
        Self {
            action_type: ActionType::Retrieve,
            detail: true,
        }
    }
    /// Documentation for `create`
    ///
    pub fn create() -> Self {
        Self {
            action_type: ActionType::Create,
            detail: false,
        }
    }
    /// Documentation for `update`
    pub fn update() -> Self {
        Self {
            action_type: ActionType::Update,
            detail: true,
        }
    }
    /// Documentation for `partial_update`
    ///
    pub fn partial_update() -> Self {
        Self {
            action_type: ActionType::PartialUpdate,
            detail: true,
        }
    }
    /// Documentation for `destroy`
    ///
    pub fn destroy() -> Self {
        Self {
            action_type: ActionType::Destroy,
            detail: true,
        }
    }
    /// Documentation for `custom`
    ///
    pub fn custom(name: &'static str, detail: bool) -> Self {
        Self {
            action_type: ActionType::Custom(name),
            detail,
        }
    }

    /// Create an Action from a string name
    /// Maps standard action names to their corresponding ActionType
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_viewsets::Action;
    ///
    /// let action = Action::from_name("list");
    /// assert_eq!(action.detail, false);
    /// ```
    pub fn from_name(name: &str) -> Self {
        match name {
            "list" => Self::list(),
            "retrieve" => Self::retrieve(),
            "create" => Self::create(),
            "update" => Self::update(),
            "partial_update" => Self::partial_update(),
            "destroy" => Self::destroy(),
            custom_name => Self {
                action_type: ActionType::Custom(Box::leak(
                    custom_name.to_string().into_boxed_str(),
                )),
                detail: false, // Default to list-like action
            },
        }
    }
}
