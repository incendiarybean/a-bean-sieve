#[derive(Debug, Default, serde::Deserialize, serde::Serialize, Clone, Copy)]
pub enum TrafficFilterType {
    #[default]
    Allow,
    Deny,
}

impl ToString for TrafficFilterType {
    fn to_string(&self) -> String {
        match self {
            TrafficFilterType::Allow => String::from("Allow"),
            TrafficFilterType::Deny => String::from("Deny"),
        }
    }
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct TrafficFilterList {
    pub allow_exclusions: Vec<String>,
    pub deny_exclusions: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct TrafficFilter {
    filter_enabled: bool,
    filter_type: TrafficFilterType,
    filter_list: TrafficFilterList,
}

impl TrafficFilter {
    pub fn default() -> Self {
        Self {
            filter_enabled: bool::default(),
            filter_type: TrafficFilterType::default(),
            filter_list: TrafficFilterList::default(),
        }
    }

    /// Returns whether the traffic filter is currently active or not.
    pub fn get_enabled(&self) -> bool {
        self.filter_enabled
    }

    /// Sets whether the traffic filter is currently active or not.
    ///
    /// # Arguments:
    /// * `active` - A bool value, whether the exclusion list is active or not.
    pub fn set_enabled(&mut self, active: bool) {
        self.filter_enabled = active
    }

    /// Returns the current exclusion type, e.g. Allow/Deny.
    pub fn get_filter_type(&self) -> TrafficFilterType {
        self.filter_type
    }

    /// Sets the current exclusion type, e.g. Allow/Deny.
    ///
    /// # Arguments:
    /// * `filter_type` - A TrafficFilterType to set the filter type to.
    pub fn set_filter_type(&mut self, filter_type: TrafficFilterType) {
        self.filter_type = filter_type;
    }

    /// Returns the opposing filter type, e.g. Allow -> Deny.
    pub fn get_opposing_filter_type(&self) -> TrafficFilterType {
        match self.get_filter_type() {
            TrafficFilterType::Allow => TrafficFilterType::Deny,
            TrafficFilterType::Deny => TrafficFilterType::Allow,
        }
    }

    /// Returns the current exclusion list.
    pub fn get_filter_list(&self) -> Vec<String> {
        match self.get_filter_type() {
            TrafficFilterType::Allow => self.filter_list.allow_exclusions.clone(),
            TrafficFilterType::Deny => self.filter_list.deny_exclusions.clone(),
        }
    }

    /// Returns the current exclusion list as a mutable reference.
    pub fn get_filter_list_mut(&mut self) -> &mut Vec<String> {
        match self.get_filter_type() {
            TrafficFilterType::Allow => self.filter_list.allow_exclusions.as_mut(),
            TrafficFilterType::Deny => self.filter_list.deny_exclusions.as_mut(),
        }
    }

    /// Sets the exclusion list you're currently using.
    ///
    /// # Arguments:
    /// * `list` - A Vec<String> of URIs to set the current exclusion list to.
    pub fn set_filter_list(&mut self, list: Vec<String>) {
        match self.get_filter_type() {
            TrafficFilterType::Allow => self.filter_list.allow_exclusions = list,
            TrafficFilterType::Deny => self.filter_list.deny_exclusions = list,
        }
    }

    /// Add/Remove an item in the current filter list.
    ///     
    /// # Arguments:
    /// * `value` - A String to add to/remove from the current exclusion list.
    pub fn update_filter_list(&mut self, value: String) {
        if self.in_filter_list(&value) {
            self.get_filter_list_mut().retain(|item| item != &value);
        } else {
            self.get_filter_list_mut().push(value);
        }
    }

    /// Updates a specific item in the current exclusion list.
    ///
    /// # Arguments:
    /// * `index` - A usize indicating the position of the value to update in the current exclusion list.
    /// * `value` - A String to update the existing record in the current exclusion list to.
    pub fn update_filter_list_item(&mut self, index: usize, value: String) {
        self.get_filter_list_mut()[index] = value;
    }

    /// Returns whether the provided URI is in the exclusion list.
    ///
    /// # Arguments:
    /// * `uri` - A str to check the current exclusion list for.
    pub fn in_filter_list(&self, uri: &String) -> bool {
        self.get_filter_list()
            .iter()
            .any(|item| uri.contains(item) || item.contains(*&uri))
    }

    /// Returns whether we're blocking by exclusion, or allowing by exclusion.
    pub fn is_blocking(&self) -> bool {
        match self.get_filter_type() {
            TrafficFilterType::Allow => false,
            TrafficFilterType::Deny => true,
        }
    }
}
