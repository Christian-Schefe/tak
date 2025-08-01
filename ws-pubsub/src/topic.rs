use std::{collections::HashMap, str::Split};

pub fn topic_matches(filter: &str, topic: &str) -> bool {
    topic_matches_iter(filter.split('/'), topic.split('/'))
}

pub fn topic_matches_iter<'a, 'b, F, T>(filter: F, topic: T) -> bool
where
    F: IntoIterator<Item = &'a str>,
    T: IntoIterator<Item = &'b str>,
{
    let mut filter = filter.into_iter().peekable();
    let mut topic = topic.into_iter().peekable();

    loop {
        match (filter.next(), topic.next()) {
            (None, None) => return true,
            (Some("#"), _) => return true,
            (Some("+"), Some(_)) => (),
            (Some(filter), Some(topic)) if filter == topic => (),
            _ => return false,
        }
    }
}

#[derive(Debug)]
struct Node<T> {
    value: Option<(String, T)>,

    children: HashMap<Box<str>, Node<T>>,
}

impl<T> Node<T> {
    fn is_empty(&self) -> bool {
        self.value.is_none() && self.children.is_empty()
    }

    fn value_ref(&self) -> Option<(&str, &T)> {
        self.value.as_ref().map(|(k, v)| (k.as_str(), v))
    }

    fn iter(&self) -> NodeIter<T> {
        Box::new(
            self.value
                .iter()
                .map(|(k, v)| (k.as_str(), v))
                .chain(self.children.values().flat_map(|n| n.iter())),
        )
    }

    fn iter_mut(&mut self) -> NodeIterMut<T> {
        Box::new(
            self.value
                .iter_mut()
                .map(|(k, v)| (k.as_str(), v))
                .chain(self.children.values_mut().flat_map(|n| n.iter_mut())),
        )
    }

    fn prune(&mut self) {
        for node in &mut self.children.values_mut() {
            node.shrink_to_fit();
        }

        self.children.retain(|_, node| !node.is_empty());
    }

    fn shrink_to_fit(&mut self) {
        for node in self.children.values_mut() {
            node.shrink_to_fit();
        }

        self.children.retain(|_, node| !node.is_empty());
        self.children.shrink_to_fit();
    }
}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Node {
            value: None,
            children: HashMap::new(),
        }
    }
}

type NodeIter<'a, T> = Box<dyn Iterator<Item = (&'a str, &'a T)> + 'a>;

type NodeIterMut<'a, T> = Box<dyn Iterator<Item = (&'a str, &'a mut T)> + 'a>;

#[derive(Debug)]
pub struct TopicMatcher<T> {
    root: Node<T>,
}

impl<T> TopicMatcher<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_empty()
    }

    pub fn clear(&mut self) {
        self.root = Node::default();
    }

    pub fn insert<S: Into<String>>(&mut self, filter: S, val: T) {
        let filter = filter.into();
        let mut curr = &mut self.root;

        for field in filter.split('/') {
            curr = curr.children.entry(field.into()).or_default()
        }
        curr.value = Some((filter, val));
    }

    pub fn get(&self, filter: &str) -> Option<&T> {
        self.get_key_value(filter).map(|(_, v)| v)
    }

    pub fn get_key_value(&self, filter: &str) -> Option<(&str, &T)> {
        let mut curr = &self.root;

        for field in filter.split('/') {
            curr = match curr.children.get(field) {
                Some(node) => node,
                None => return None,
            };
        }
        curr.value.as_ref().map(|(k, v)| (k.as_str(), v))
    }

    pub fn get_mut(&mut self, filter: &str) -> Option<&mut T> {
        let mut curr = &mut self.root;

        for field in filter.split('/') {
            curr = match curr.children.get_mut(field) {
                Some(node) => node,
                None => return None,
            };
        }
        curr.value.as_mut().map(|(_, v)| v)
    }

    pub fn remove(&mut self, filter: &str) -> Option<T> {
        let mut curr = &mut self.root;

        for field in filter.split('/') {
            curr = match curr.children.get_mut(field) {
                Some(node) => node,
                None => return None,
            };
        }
        curr.value.take().map(|(_, v)| v)
    }

    pub fn prune(&mut self) {
        self.root.prune()
    }

    pub fn shrink_to_fit(&mut self) {
        self.root.shrink_to_fit()
    }

    pub fn iter(&self) -> NodeIter<T> {
        self.root.iter()
    }

    pub fn iter_mut(&mut self) -> NodeIterMut<T> {
        self.root.iter_mut()
    }

    pub fn matches<'a, 'b>(&'a self, topic: &'b str) -> MatchIter<'a, 'b, T> {
        MatchIter::new(&self.root, topic)
    }

    pub fn has_match(&self, topic: &str) -> bool {
        self.matches(topic).next().is_some()
    }
}

impl<T: Clone> TopicMatcher<T> {
    pub fn insert_many<S: AsRef<str>>(&mut self, filters: &[S], val: T) {
        for filter in filters {
            self.insert(filter.as_ref(), val.clone());
        }
    }
}

impl<T> Default for TopicMatcher<T> {
    fn default() -> Self {
        TopicMatcher {
            root: Node::default(),
        }
    }
}

impl<T> From<HashMap<&str, T>> for TopicMatcher<T> {
    fn from(mut m: HashMap<&str, T>) -> Self {
        let mut matcher = Self::new();
        for (filt, val) in m.drain() {
            matcher.insert(filt, val);
        }
        matcher
    }
}

impl<T> From<HashMap<String, T>> for TopicMatcher<T> {
    fn from(mut m: HashMap<String, T>) -> Self {
        let mut matcher = Self::new();
        for (filt, val) in m.drain() {
            matcher.insert(filt, val);
        }
        matcher
    }
}

impl<'a, T: 'a> IntoIterator for &'a TopicMatcher<T> {
    type Item = (&'a str, &'a T);
    type IntoIter = NodeIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: 'a> IntoIterator for &'a mut TopicMatcher<T> {
    type Item = (&'a str, &'a mut T);
    type IntoIter = NodeIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Debug)]
pub struct MatchIter<'a, 'b, T> {
    remaining: Vec<(&'a Node<T>, Split<'b, char>)>,
}

impl<'a, 'b, T> MatchIter<'a, 'b, T> {
    fn new(node: &'a Node<T>, topic: &'b str) -> Self {
        let fields = topic.split('/');
        Self {
            remaining: vec![(node, fields)],
        }
    }
}

impl<'a, 'b, T> Iterator for MatchIter<'a, 'b, T> {
    type Item = (&'a str, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let (node, mut fields) = self.remaining.pop()?;

        let field = match fields.next() {
            Some(field) => field,
            None => {
                return node
                    .value
                    .as_ref()
                    .map(|(k, v)| (k.as_str(), v))
                    .or_else(|| {
                        node.children
                            .get("#")
                            .and_then(|child| child.value_ref())
                            .or_else(|| self.next())
                    });
            }
        };

        if let Some(child) = node.children.get(field) {
            self.remaining.push((child, fields.clone()));
        }

        if let Some(child) = node.children.get("+") {
            self.remaining.push((child, fields));
        }

        if let Some(child) = node.children.get("#") {
            return child.value_ref();
        }

        self.next()
    }
}
