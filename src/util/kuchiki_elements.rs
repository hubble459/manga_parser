use itertools::Itertools;
use kuchiki::{
    iter::{Descendants, Elements, Select},
    traits::NodeIterator,
};

pub trait ElementsTrait {
    fn len(&self) -> usize;

    fn own_text(&self) -> String;

    fn all_text(&self, join_str: &str) -> String;

    fn attr_first_of(&self, attrs: &[String]) -> Option<String>;

    fn attr(&self, attr: &str) -> Option<String>;

    fn attrs(&self, attr: &str) -> Vec<String>;

    fn attrs_first_of(&self, attrs: &[String]) -> Vec<String>;
}

impl ElementsTrait for kuchiki::NodeRef {
    fn len(&self) -> usize {
        self.descendants().count()
    }

    fn own_text(&self) -> String {
        self.descendants()
            .map(|el| {
                let mut s = String::new();
                for text_node in el.children().text_nodes() {
                    s.push_str(&text_node.borrow());
                }
                s
            })
            .join("\n")
            .trim()
            .to_string()
    }

    fn all_text(&self, join_str: &str) -> String {
        self.descendants()
            .map(|el| el.text_contents())
            .join(join_str)
            .trim()
            .to_string()
    }

    fn attr_first_of(&self, attrs: &[String]) -> Option<String> {
        for attr in attrs {
            let val = self.attr(attr);
            if val.is_some() {
                return val;
            }
        }
        return None;
    }

    fn attr(&self, attr: &str) -> Option<String> {
        let cloned = self.descendants();
        for node in cloned.into_iter() {
            if let Some(node) = node.as_element() {
                let attributes = node.attributes.borrow();
                let val = attributes.get(attr);
                if let Some(val) = val {
                    return Some(val.to_string());
                }
            }
        }
        return None;
    }

    fn attrs(&self, attr: &str) -> Vec<String> {
        let cloned = self.descendants();
        let mut attrs = vec![];

        for node in cloned.into_iter() {
            if let Some(node) = node.as_element() {
                let attributes = node.attributes.borrow();
                let val = attributes.get(attr);
                if let Some(val) = val {
                    attrs.push(val.to_string());
                }
            }
        }

        return attrs;
    }

    fn attrs_first_of(&self, attrs: &[String]) -> Vec<String> {
        let cloned = self.descendants();
        let mut found_attrs = vec![];

        for node in cloned.into_iter() {
            if let Some(node) = node.as_element() {
                let attributes = node.attributes.borrow();
                for attr in attrs {
                    let val = attributes.get(attr.to_string());
                    if let Some(val) = val {
                        found_attrs.push(val.to_string());
                        break;
                    }
                }
            }
        }

        return found_attrs;
    }
}

impl ElementsTrait for Select<Elements<Descendants>> {
    fn len(&self) -> usize {
        self.clone().count()
    }

    fn own_text(&self) -> String {
        self.clone()
            .map(|el| {
                let mut s = String::new();
                for text_node in el.as_node().children().text_nodes() {
                    s.push_str(&text_node.borrow());
                }
                s
            })
            .join("\n")
            .trim()
            .to_string()
    }

    fn all_text(&self, join_str: &str) -> String {
        self.clone()
            .map(|el| el.text_contents())
            .join(join_str)
            .trim()
            .to_string()
    }

    fn attr_first_of(&self, attrs: &[String]) -> Option<String> {
        for attr in attrs {
            let val = self.attr(attr);
            if val.is_some() {
                return val;
            }
        }
        return None;
    }

    fn attr(&self, attr: &str) -> Option<String> {
        let cloned = self.clone();
        for node in cloned.into_iter() {
            let attributes = node.attributes.borrow();
            let val = attributes.get(attr);
            if let Some(val) = val {
                return Some(val.to_string());
            }
        }
        return None;
    }

    fn attrs(&self, attr: &str) -> Vec<String> {
        let cloned = self.clone();
        let mut attrs = vec![];

        for node in cloned.into_iter() {
            let attributes = node.attributes.borrow();
            let val = attributes.get(attr);
            if let Some(val) = val {
                attrs.push(val.to_string());
            }
        }

        return attrs;
    }

    fn attrs_first_of(&self, attrs: &[String]) -> Vec<String> {
        let cloned = self.clone();
        let mut found_attrs = vec![];

        for node in cloned.into_iter() {
            let attributes = node.attributes.borrow();
            for attr in attrs {
                let val = attributes.get(attr.to_string());
                if let Some(val) = val {
                    found_attrs.push(val.to_string());
                    break;
                }
            }
        }

        return found_attrs;
    }
}
