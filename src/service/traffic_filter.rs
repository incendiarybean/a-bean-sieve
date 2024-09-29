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

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TrafficFilter {
    pub filter_enabled: bool,
    pub filter_type: TrafficFilterType,
    pub filter_list: TrafficFilterList,
}

impl TrafficFilter {
    pub fn default() -> Self {
        Self {
            filter_enabled: false,
            filter_type: TrafficFilterType::Allow,
            filter_list: TrafficFilterList::default(),
        }
    }

    pub fn get_enabled(&self) -> bool {
        self.filter_enabled
    }

    pub fn set_enabled(&mut self, value: bool) {
        self.filter_enabled = value
    }

    pub fn get_filter_type(&self) -> TrafficFilterType {
        self.filter_type
    }

    pub fn set_filter_type(&mut self, value: TrafficFilterType) {
        self.filter_type = value;
    }

    pub fn get_opposing_filter_type(&self) -> TrafficFilterType {
        match self.get_filter_type() {
            TrafficFilterType::Allow => TrafficFilterType::Deny,
            TrafficFilterType::Deny => TrafficFilterType::Allow,
        }
    }

    pub fn get_filter_list(&self) -> Vec<String> {
        match self.get_filter_type() {
            TrafficFilterType::Allow => self.filter_list.allow_exclusions.clone(),
            TrafficFilterType::Deny => self.filter_list.deny_exclusions.clone(),
        }
    }

    pub fn get_filter_list_mut(&mut self) -> &mut Vec<String> {
        match self.get_filter_type() {
            TrafficFilterType::Allow => self.filter_list.allow_exclusions.as_mut(),
            TrafficFilterType::Deny => self.filter_list.deny_exclusions.as_mut(),
        }
    }

    pub fn set_filter_list(&mut self, list: Vec<String>) {
        match self.get_filter_type() {
            TrafficFilterType::Allow => self.filter_list.allow_exclusions = list,
            TrafficFilterType::Deny => self.filter_list.deny_exclusions = list,
        }
    }

    pub fn update_filter_list(&mut self, value: String) {
        if self.in_filter_list(&value) {
            self.get_filter_list_mut().retain(|item| item != &value);
        } else {
            self.get_filter_list_mut().push(value);
        }
    }

    pub fn update_filter_list_item(&mut self, index: usize, value: String) {
        self.get_filter_list_mut()[index] = value;
    }

    pub fn in_filter_list(&self, uri: &String) -> bool {
        self.get_filter_list()
            .iter()
            .any(|item| uri.contains(item) || item.contains(*&uri))
    }
}
