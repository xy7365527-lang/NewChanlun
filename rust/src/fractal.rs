//! 分型识别（Fractal Detection）— 逐位等价移植自 src/newchan/a_fractal.py
//!
//! 分型是一维价格函数的局部极值点。双条件判定：
//! - 顶分型：h_curr > h_prev AND h_curr > h_next AND l_curr > l_prev AND l_curr > l_next
//! - 底分型：l_curr < l_prev AND l_curr < l_next AND h_curr < h_prev AND h_curr < h_next

/// 分型类型。对应 Python Literal["top", "bottom"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalKind {
    Top,
    Bottom,
}

/// 一个分型。对应 Python `Fractal` frozen dataclass。
///
/// `price` 用 f64 存储；相等性比较时价格用位精确比较（移植自 Python 的
/// `!=` 对 Fractal 的字段比较——dataclass eq 对 float 是精确相等）。
#[derive(Debug, Clone, Copy)]
pub struct Fractal {
    /// 分型中间 K 线在 df_merged 中的位置索引（0-based iloc）。
    pub idx: usize,
    pub kind: FractalKind,
    /// 分型极值：顶分型 = high，底分型 = low。
    pub price: f64,
}

impl PartialEq for Fractal {
    /// 复刻 Python frozen dataclass 的 `==`：所有字段精确相等。
    /// 注意 price 用位精确比较（`to_bits`）以严格对齐 dataclass eq——
    /// Python `__eq__` 对 float 用 `==`，对正常价格等价于位比较。
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
            && self.kind == other.kind
            && self.price == other.price
    }
}

/// 对单个位置做双条件分型判定，返回 Some(Fractal) 或 None。
///
/// 逐位等价移植自 `a_fractal._classify_fractal`。
#[allow(clippy::too_many_arguments)]
pub fn classify_fractal(
    h_prev: f64,
    h_curr: f64,
    h_next: f64,
    l_prev: f64,
    l_curr: f64,
    l_next: f64,
    idx: usize,
) -> Option<Fractal> {
    if h_curr > h_prev && h_curr > h_next && l_curr > l_prev && l_curr > l_next {
        return Some(Fractal {
            idx,
            kind: FractalKind::Top,
            price: h_curr,
        });
    }
    if l_curr < l_prev && l_curr < l_next && h_curr < h_prev && h_curr < h_next {
        return Some(Fractal {
            idx,
            kind: FractalKind::Bottom,
            price: l_curr,
        });
    }
    None
}
