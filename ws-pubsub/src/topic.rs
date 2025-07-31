use crate::ServerMessage;

pub struct StaticParamsTopic<T, const N: usize, const M: usize> {
    pub parts: [Option<(usize, usize)>; N],
    pub path: &'static str,
    pub _marker: std::marker::PhantomData<T>,
}

pub mod topic_helpers {
    pub const fn count_parts(path: &str) -> usize {
        let mut count = 1;
        let bytes = path.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'/' {
                count += 1;
            }
            i += 1;
        }
        count
    }

    pub const fn count_wildcards(path: &str) -> usize {
        let mut count = 0;
        let bytes = path.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'+'
                && (i == 0 || bytes[i - 1] == b'/')
                && (i + 1 == bytes.len() || bytes[i + 1] == b'/')
            {
                count += 1;
            }
            i += 1;
        }
        count
    }

    pub const fn build_parts<const N: usize, const M: usize>(
        path: &str,
    ) -> [Option<(usize, usize)>; N] {
        let mut arr: [Option<(usize, usize)>; N] = [None; N];
        let bytes = path.as_bytes();
        let mut i = 0;
        let mut part_index = 0;
        let mut start = 0;
        while i < bytes.len() {
            if bytes[i] == b'/' {
                arr[part_index] =
                    if i >= 1 && bytes[i - 1] == b'+' && (i == 1 || bytes[i - 2] == b'/') {
                        None
                    } else {
                        Some((start, i))
                    };
                part_index += 1;
                start = i + 1;
            }
            i += 1;
        }
        arr[part_index] = if i >= 1 && bytes[i - 1] == b'+' && (i == 1 || bytes[i - 2] == b'/') {
            None
        } else {
            Some((start, i))
        };
        arr
    }
}

#[macro_export]
macro_rules! static_params_topic {
    ($ty:ty, $path:expr) => {{
        const N: usize = $crate::topic::topic_helpers::count_parts($path);
        const M: usize = $crate::topic::topic_helpers::count_wildcards($path);

        $crate::topic::StaticParamsTopic::<$ty, N, M> {
            parts: $crate::topic::topic_helpers::build_parts::<N, M>($path),
            path: $path,
            _marker: std::marker::PhantomData,
        }
    }};
}

#[macro_export]
macro_rules! static_topic {
    ($name:ident, $ty:ty, $path:expr) => {
        static $name: $crate::topic::StaticParamsTopic<
            $ty,
            { $crate::topic::topic_helpers::count_parts($path) },
            { $crate::topic::topic_helpers::count_wildcards($path) },
        > = $crate::static_params_topic!($ty, $path);
    };
}

static_topic!(test_topic3, usize, "foo/+/bar");

static test_topic: StaticParamsTopic<usize, 3, 1> = static_params_topic!(usize, "foo/+/bar");
static test_topic2: StaticParamsTopic<usize, 5, 2> = static_params_topic!(usize, "foo/+/+/++/bar");

impl<T, const N: usize, const M: usize> StaticParamsTopic<T, N, M> {
    pub fn try_extract<'a>(&self, topic: &'a str) -> Option<[&'a str; M]> {
        let parts = topic.split('/');
        let mut result = [""; M];
        let mut j = 0;
        for (i, part) in parts.enumerate() {
            if let Some(self_part) = self.parts[i] {
                let self_part = &self.path[self_part.0..self_part.1];
                if part != self_part {
                    return None;
                }
            } else {
                result[j] = part;
                j += 1;
            }
        }
        Some(result)
    }
}

impl<T, const N: usize, const M: usize> TopicTrait<T> for StaticParamsTopic<T, N, M> {
    fn name(&self) -> &str {
        self.path
    }
}

pub struct StaticTopic<T> {
    pub name: &'static str,
    _marker: std::marker::PhantomData<T>,
}

impl<T> StaticTopic<T> {
    pub const fn new(name: &'static str) -> Self {
        StaticTopic {
            name,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> TopicTrait<T> for StaticTopic<T> {
    fn name(&self) -> &str {
        self.name
    }
}

#[derive(Debug, Clone)]
pub struct Topic<T> {
    pub name: String,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Topic<T> {
    pub fn new(name: impl AsRef<str>) -> Self {
        Topic {
            name: name.as_ref().to_string(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> TopicTrait<T> for Topic<T> {
    fn name(&self) -> &str {
        &self.name
    }
}

pub trait TopicTrait<T> {
    fn name(&self) -> &str;

    fn make_server_message(&self, payload: T) -> Result<ServerMessage, serde_json::Error>
    where
        T: serde::Serialize + Send + 'static,
    {
        let payload = serde_json::to_value(payload)?;
        Ok(ServerMessage {
            topic: self.name().to_string(),
            payload,
        })
    }
}
