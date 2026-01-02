use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeDomain {
    Point,
    Vertex,
    Primitive,
    Detail,
}

impl AttributeDomain {
    pub const ALL: [AttributeDomain; 4] = [
        AttributeDomain::Vertex,
        AttributeDomain::Point,
        AttributeDomain::Primitive,
        AttributeDomain::Detail,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    Float,
    Int,
    Vec2,
    Vec3,
    Vec4,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeStorage {
    Float(Vec<f32>),
    Int(Vec<i32>),
    Vec2(Vec<[f32; 2]>),
    Vec3(Vec<[f32; 3]>),
    Vec4(Vec<[f32; 4]>),
}

impl AttributeStorage {
    pub fn len(&self) -> usize {
        match self {
            AttributeStorage::Float(values) => values.len(),
            AttributeStorage::Int(values) => values.len(),
            AttributeStorage::Vec2(values) => values.len(),
            AttributeStorage::Vec3(values) => values.len(),
            AttributeStorage::Vec4(values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn data_type(&self) -> AttributeType {
        match self {
            AttributeStorage::Float(_) => AttributeType::Float,
            AttributeStorage::Int(_) => AttributeType::Int,
            AttributeStorage::Vec2(_) => AttributeType::Vec2,
            AttributeStorage::Vec3(_) => AttributeType::Vec3,
            AttributeStorage::Vec4(_) => AttributeType::Vec4,
        }
    }

    pub fn as_ref(&self) -> AttributeRef<'_> {
        match self {
            AttributeStorage::Float(values) => AttributeRef::Float(values.as_slice()),
            AttributeStorage::Int(values) => AttributeRef::Int(values.as_slice()),
            AttributeStorage::Vec2(values) => AttributeRef::Vec2(values.as_slice()),
            AttributeStorage::Vec3(values) => AttributeRef::Vec3(values.as_slice()),
            AttributeStorage::Vec4(values) => AttributeRef::Vec4(values.as_slice()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeError {
    InvalidDomain,
    InvalidLength {
        expected: usize,
        actual: usize,
    },
    InvalidType {
        expected: AttributeType,
        actual: AttributeType,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeInfo {
    pub name: String,
    pub domain: AttributeDomain,
    pub data_type: AttributeType,
    pub len: usize,
    pub implicit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttributeRef<'a> {
    Float(&'a [f32]),
    Int(&'a [i32]),
    Vec2(&'a [[f32; 2]]),
    Vec3(&'a [[f32; 3]]),
    Vec4(&'a [[f32; 4]]),
}

impl<'a> AttributeRef<'a> {
    pub fn len(&self) -> usize {
        match self {
            AttributeRef::Float(values) => values.len(),
            AttributeRef::Int(values) => values.len(),
            AttributeRef::Vec2(values) => values.len(),
            AttributeRef::Vec3(values) => values.len(),
            AttributeRef::Vec4(values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn data_type(&self) -> AttributeType {
        match self {
            AttributeRef::Float(_) => AttributeType::Float,
            AttributeRef::Int(_) => AttributeType::Int,
            AttributeRef::Vec2(_) => AttributeType::Vec2,
            AttributeRef::Vec3(_) => AttributeType::Vec3,
            AttributeRef::Vec4(_) => AttributeType::Vec4,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshAttributes {
    point: HashMap<String, AttributeStorage>,
    vertex: HashMap<String, AttributeStorage>,
    primitive: HashMap<String, AttributeStorage>,
    detail: HashMap<String, AttributeStorage>,
}

impl MeshAttributes {
    pub fn map(&self, domain: AttributeDomain) -> &HashMap<String, AttributeStorage> {
        match domain {
            AttributeDomain::Point => &self.point,
            AttributeDomain::Vertex => &self.vertex,
            AttributeDomain::Primitive => &self.primitive,
            AttributeDomain::Detail => &self.detail,
        }
    }

    pub fn map_mut(&mut self, domain: AttributeDomain) -> &mut HashMap<String, AttributeStorage> {
        match domain {
            AttributeDomain::Point => &mut self.point,
            AttributeDomain::Vertex => &mut self.vertex,
            AttributeDomain::Primitive => &mut self.primitive,
            AttributeDomain::Detail => &mut self.detail,
        }
    }

    pub fn get(&self, domain: AttributeDomain, name: &str) -> Option<&AttributeStorage> {
        self.map(domain).get(name)
    }

    pub fn remove(&mut self, domain: AttributeDomain, name: &str) -> Option<AttributeStorage> {
        self.map_mut(domain).remove(name)
    }
}
