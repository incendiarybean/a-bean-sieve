#[derive(Debug, Default, serde::Deserialize, serde::Serialize, Clone, Copy)]
pub enum TrafficFilterType {
    #[default]
    Allow,
    Deny,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct TrafficFilterList {
    pub allow: Vec<String>,
    pub deny: Vec<String>,
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

    pub fn set_enabled(&mut self, value: bool) {
        self.filter_enabled = value
    }

    pub fn get_enabled(&self) -> bool {
        self.filter_enabled
    }

    pub fn get_filter(&self) -> TrafficFilterType {
        self.filter_type
    }

    pub fn get_filter_list(&self) -> Vec<String> {
        match self.clone().get_filter() {
            TrafficFilterType::Allow => self.filter_list.deny.clone(),
            TrafficFilterType::Deny => self.filter_list.allow.clone(),
        }
    }

    pub fn set_filter_list(&mut self, list: Vec<String>) {
        match self.clone().get_filter() {
            TrafficFilterType::Allow => self.filter_list.allow = list,
            TrafficFilterType::Deny => self.filter_list.deny = list,
        }
    }

    pub fn in_filter_list(&self, uri: String) -> bool {
        self.get_filter_list()
            .iter()
            .any(|item| uri.contains(item) || item.contains(&uri))
    }
}
