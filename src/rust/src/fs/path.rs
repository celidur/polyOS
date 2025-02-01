use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

#[derive(Debug, Clone)]
pub struct Path {
    pub components: Vec<String>,
}

impl Path {
    pub fn new(path: &str) -> Self {
        Self {
            components: path
                .split('/')
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect(),
        }
    }

    pub fn parent(&self) -> Self {
        match self.components.len() {
            0 | 1 => Self {
                components: Vec::new(),
            },
            _ => Self {
                components: self.components[..self.components.len() - 1].to_vec(),
            },
        }
    }

    pub fn as_string(&self) -> String {
        format!("/{}", self.components.join("/"))
    }

    pub fn relative_to(&self, parent: &str) -> Option<Self> {
        let self_str = self.as_string();
        self_str.strip_prefix(parent).map(Self::new)
    }
}
