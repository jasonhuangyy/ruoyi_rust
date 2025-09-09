#[derive(serde::Serialize)]
pub struct TableDataInfo<T> {
    pub total: i64,
    pub rows: Vec<T>,
    pub code: u16,
    pub msg: String,
}


impl<T> TableDataInfo<T> {
    /// 创建一个新的 TableDataInfo 实例
    /// 这是一个关联函数 (associated function)，类似于其他语言的静态方法
    ///
    /// # Arguments
    /// * `rows` - 当前页的数据列表
    /// * `total` - 查询结果的总条目数
    ///
    /// # Returns
    /// 一个初始化好的 `TableDataInfo<T>` 实例
    pub fn new(rows: Vec<T>, total: i64) -> Self {
        Self {
            rows,
            total,
            code: 200, // 默认成功码为 200
            msg: "查询成功".to_string(), // 默认成功消息
        }
    }
}