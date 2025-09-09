use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::Value;

/// 通用响应体，模仿 RuoYi 的 AjaxResult
/// T 是具体的业务数据类型，必须也能被序列化
pub struct AjaxResult<T: Serialize> {
    pub code: u16,
    pub msg: String,
    // 业务数据
    pub data: T,
}

impl<T: Serialize> AjaxResult<T> {
    /// 创建一个成功的响应，包含业务数据
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            msg: "操作成功".to_string(),
            data,
        }
    }

    /// 创建一个成功的响应，不包含业务数据
    /// `()` 在 Rust 中是一个有效的类型，并且序列化为空
    pub fn success_msg(msg: &str) -> AjaxResult<()> {
        AjaxResult {
            code: 200,
            msg: msg.to_string(),
            data: (),
        }
    }
    /// 创建一个失败的响应
    /// 这里的泛型 T 通常是 `()`，因为失败时没有业务数据
    pub fn error(code: u16, msg: &str) -> AjaxResult<()> {
        AjaxResult {
            code,
            msg: msg.to_string(),
            data: (),
        }
    }
}
 
impl<T: Serialize> Serialize for AjaxResult<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 首先，将业务数据 T 序列化成一个 serde_json::Value ,这样就可以把它当成一个 JSON 对象来处理
        let data_value = serde_json::to_value(&self.data)
            .map_err(serde::ser::Error::custom)?;

        // 期望业务数据 T 是一个 JSON 对象 (struct),如果是其他类型（如 String, Number, Array），直接将其作为 "data" 字段
        if let Value::Object(data_map) = data_value {
            // 如果 T 是一个对象，将其字段“拍平”到顶层, 估算最终 map 的大小 (code + msg + data的所有字段)
            let mut map = serializer.serialize_map(Some(2 + data_map.len()))?;

            // 序列化固定的 code 和 msg 字段
            map.serialize_entry("code", &self.code)?;
            map.serialize_entry("msg", &self.msg)?;

            // 遍历业务数据的所有字段，并将它们添加到顶层 map 中 
            for (k, v) in data_map {
                map.serialize_entry(&k, &v)?;
            }
            map.end()
        } else {
            // 如果 T 不是一个对象 (例如直接返回一个 Vec<T> 或 String), 遵循标准的 { code, msg, data } 结构
            let mut map = serializer.serialize_map(Some(3))?;
            map.serialize_entry("code", &self.code)?;
            map.serialize_entry("msg", &self.msg)?;
            map.serialize_entry("data", &self.data)?;
            map.end()
        }
    }
}