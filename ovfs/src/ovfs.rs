use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct OVFSTree {
    mountpoint: PathBuf,
    work_directory: PathBuf,
    upper: Box<OVFSNode>,
}

#[derive(Debug, Clone)]
pub(crate) struct OVFSNode {
    path: PathBuf,
    child: Option<Box<Self>>,
}

impl OVFSTree {
    pub fn new(mountpoint: PathBuf, work_directory: PathBuf, overwrite: PathBuf) -> Self {
        let upper = Box::new(OVFSNode::new(overwrite));

        Self {
            mountpoint,
            work_directory,
            upper,
        }
    }

    pub fn with_lower(&mut self, lower: PathBuf) {
        let bottom = self.upper.as_mut().last();
        bottom.put_child(lower);
    }
}

enum MaybeLast<'a> {
    Last(&'a mut OVFSNode),
    NotLast(&'a mut OVFSNode),
}

impl OVFSNode {
    pub fn new(path: PathBuf) -> Self {
        Self { path, child: None }
    }

    fn maybe_last(&mut self) -> MaybeLast {
        match self.child.is_none() {
            false => MaybeLast::NotLast(self.child.as_mut().unwrap()),
            true => MaybeLast::Last(self),
        }
    }

    pub fn last(mut self: &mut Self) -> &'_ mut OVFSNode {
        loop {
            match { self }.maybe_last() {
                MaybeLast::NotLast(child) => self = child,
                MaybeLast::Last(end) => return end,
            }
        }
    }

    pub fn put_child(&mut self, child: PathBuf) {
        self.child = Some(Box::new(Self::new(child)));
    }
}

impl IntoIterator for Box<OVFSNode> {
    type Item = PathBuf;
    type IntoIter = OVFSNodeIter;

    fn into_iter(self) -> OVFSNodeIter {
        // let current = ;
        Self::IntoIter {
            current: Some(self),
        }
    }
}

pub struct OVFSNodeIter {
    current: Option<Box<OVFSNode>>,
}

impl Iterator for OVFSNodeIter {
    type Item = PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        match self.current.is_none() {
            false => {
                let out = self.current.as_ref().unwrap().path.clone();
                let next = self.current.as_ref().unwrap().child.clone();
                *self = OVFSNodeIter { current: next };
                Some(out)
            }
            true => None,
        }
    }
}
