# Pretext Rust + egui 完整实现方案

参考实现：https://github.com/chenglou/pretext（只读，TypeScript）
目标：行为对等的原生桌面版本，egui = "0.33.3"
验收标准：行为对等 + 稳定几何，不要求像素级 CSS 匹配

---

## 一、依赖版本

```toml
[workspace.dependencies]
# GUI
eframe                = "0.33.3"   # 内部依赖 egui ^0.33.3 / wgpu ^27.0.1 / winit ^0.30.12
egui                  = "0.33.3"

# 字形 / 字体
rustybuzz             = "0.20.1"   # OpenType shaping
ttf-parser            = "0.25.1"   # face 解析，与 rustybuzz 同版本系列
fontdb                = "0.23.0"   # CSS 风格字体匹配

# Unicode 文字处理
unicode-segmentation  = "1.13.2"   # grapheme / word 边界
unicode-bidi          = "0.3.18"   # BidiInfo 段落分析
unicode-linebreak     = "0.1.5"    # UAX #14 baseline
unicode-script        = "0.5.7"    # char → Script 属性

# SVG 栅格化
resvg                 = "0.47.0"
tiny-skia             = "0.12.0"   # resvg 底层，alpha-hull 像素遍历

# 图像
image                 = "0.25.10"  # Pixmap → egui ColorImage

# 工具
lru                   = "0.16.3"   # ShapeCache LRU
ahash                 = "0.8.12"   # 快速哈希
serde                 = { version = "1.0.228", features = ["derive"] }
serde_json            = "1.0.149"  # masonry.json
parking_lot           = "0.12.5"   # Mutex / RwLock
bytemuck              = "1.25.0"   # 像素 buffer 转换
```

> eframe 0.33.3 与 0.34.x 之间有 breaking change，整套方案以 0.33.3 为准。

---

## 二、Workspace 结构

```
pretext-rs/
├── Cargo.toml                      # workspace, resolver = "2"
├── Cargo.lock                      # 提交到 VCS
│
├── crates/
│   └── pretext/                    # 纯计算库，零 UI 依赖
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── engine.rs           # PretextEngine 公共入口
│       │   ├── analysis.rs         # 空白规范化、URL/数字/标点合并、CJK 规则
│       │   ├── bidi.rs             # BidiInfo → BidiRun 列表
│       │   ├── font_catalog.rs     # fontdb 封装 + coverage_map + fallback 链
│       │   ├── measure.rs          # rustybuzz shaping + ShapeCache
│       │   ├── line_break.rs       # UAX14 baseline + override pipeline
│       │   └── layout.rs           # 四接口实现 + ParagraphCache（P2）
│       └── tests/
│           ├── shaping.rs
│           ├── font_fallback.rs
│           ├── whitespace.rs
│           ├── segmentation.rs
│           ├── line_break.rs
│           ├── layout_parity.rs
│           ├── bidi.rs
│           └── goldens/            # 12 个 JSON golden 文件
│
└── demos/
    └── app/
        ├── Cargo.toml
        ├── build.rs                # 编译期打包 assets 到二进制
        ├── assets/
        │   ├── fonts/
        │   │   ├── NotoSans-Regular.ttf
        │   │   ├── NotoSansCJK-Regular.ttc
        │   │   ├── NotoSansMyanmar-Regular.ttf
        │   │   ├── NotoEmoji-Regular.ttf
        │   │   └── NotoSansMono-Regular.ttf
        │   ├── logos/              # SVG logo（dynamic_layout 用）
        │   └── masonry.json
        └── src/
            ├── main.rs
            ├── app.rs              # PretextDemoApp，帧循环
            ├── assets.rs           # 字体注册、SVG 纹理缓存
            ├── geometry.rs         # wrap-geometry 移植，alpha-hull
            └── demos/
                ├── mod.rs          # DemoWindow trait
                ├── catalog.rs
                ├── accordion.rs
                ├── bubbles.rs
                ├── dynamic_layout.rs
                ├── editorial_engine.rs
                ├── rich_note.rs
                ├── variable_typographic_ascii.rs
                └── masonry.rs
```

---

## 三、引擎核心设计

### 3.1 公共 API

完全对应 JS 参考实现的函数命名：

```rust
// crates/pretext/src/engine.rs

pub struct PretextEngine {
    font_catalog: FontCatalog,
    shape_cache:  ShapeCache,
    para_cache:   Option<ParagraphCache>,   // P2 加入
    locale:       Option<String>,
}

/// 对应 JS TextStyleSpec（JS 用 CSS font shorthand "16px Inter"，
/// Rust 拆为结构体避免解析 CSS 字符串）
pub struct TextStyleSpec {
    pub families: Vec<String>,   // 按优先级，引擎自动 fallback
    pub size_px:  f32,
    pub weight:   u16,           // 100–900
    pub italic:   bool,
}

/// TextStyleSpec 的 Hash 实现：size_px 量化避免浮点 cache miss
impl Hash for TextStyleSpec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.families.hash(state);
        ((self.size_px * 64.0).round() as u32).hash(state);  // 2^6 精度
        self.weight.hash(state);
        self.italic.hash(state);
    }
}

pub struct PrepareOptions {
    pub white_space: WhiteSpaceMode,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum WhiteSpaceMode {
    Normal,   // 折叠空白，软换行
    PreWrap,  // 保留空白 + \t \n，tab_size = 8（对齐 JS 默认，非 CSS 默认 4）
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutCursor {
    pub segment_index:  usize,
    pub grapheme_index: usize,
}

pub struct LayoutLine {
    pub text:  String,
    pub width: f32,
    pub start: LayoutCursor,
    pub end:   LayoutCursor,
}

pub struct LayoutLineRange {
    pub width: f32,
    pub start: LayoutCursor,
    pub end:   LayoutCursor,
}

pub struct LayoutResult {
    pub height:     f32,
    pub line_count: usize,
}

pub struct LayoutWithLinesResult {
    pub height:     f32,
    pub line_count: usize,
    pub lines:      Vec<LayoutLine>,
}

impl PretextEngine {
    pub fn new() -> Self;

    // ── use-case 1：对应 JS prepare() + layout() ──────────────────
    pub fn prepare(
        &self, text: &str, style: &TextStyleSpec, opts: &PrepareOptions,
    ) -> PreparedText;

    pub fn layout(
        &self, prepared: &PreparedText, max_width: f32, line_height: f32,
    ) -> LayoutResult;

    // ── use-case 2：对应 JS prepareWithSegments() + 三种 layout ───
    pub fn prepare_with_segments(
        &self, text: &str, style: &TextStyleSpec, opts: &PrepareOptions,
    ) -> PreparedTextWithSegments;

    pub fn layout_with_lines(
        &self, prepared: &PreparedTextWithSegments,
        max_width: f32, line_height: f32,
    ) -> LayoutWithLinesResult;

    /// 对应 JS walkLineRanges()，返回最大行宽
    pub fn walk_line_ranges(
        &self, prepared: &PreparedTextWithSegments,
        max_width: f32, on_line: impl FnMut(&LayoutLineRange),
    ) -> f32;

    /// 对应 JS layoutNextLine()
    pub fn layout_next_line(
        &self, prepared: &PreparedTextWithSegments,
        start: &mut LayoutCursor, max_width: f32,
    ) -> Option<LayoutLine>;

    // ── 工具 ────────────────────────────────────────────────────────
    pub fn clear_cache(&mut self);                    // 对应 JS clearCache()
    pub fn set_locale(&mut self, locale: Option<&str>); // 对应 JS setLocale()

    // demo 额外需要（非 JS API）
    pub fn glyph_advance(&self, ch: char, style: &TextStyleSpec) -> f32;
    pub fn prefix_widths(&self, text: &str, style: &TextStyleSpec) -> Arc<[f32]>;
}
```

### 3.2 PreparedText 结构

```rust
pub struct PreparedText {
    pub(crate) text:     Arc<str>,
    pub(crate) segments: Arc<[Segment]>,
    pub(crate) hash:     u64,    // ahash，构造时一次性计算，供 ParagraphCache 用
}

// Clone 是 O(1)（Arc 引用计数）
impl Clone for PreparedText { ... }

pub struct PreparedTextWithSegments {
    pub(crate) inner:    PreparedText,
    pub(crate) seg_meta: Arc<[SegmentMeta]>,  // grapheme 边界、宽度等
}
```

### 3.3 Segment 与 SegmentKind

v1 只用 `Text` 和 `AtomicPlaceholder`，为 P2 的 InlineItem 模型预留结构位：

```rust
#[derive(Clone, Copy, Debug)]
pub enum SegmentKind {
    Text,
    AtomicPlaceholder { width: f32 },  // v1 Chip 用这个；v2 扩展 InlineBox 等
}

pub struct Segment {
    pub kind:       SegmentKind,
    pub byte_range: Range<usize>,
    pub glyphs:     Arc<[ShapedGlyph]>,  // AtomicPlaceholder 时为空
}
```

> P2 升级时只需给 `SegmentKind` 加 variant，`PreparedTextWithSegments` 签名不变，无 breaking change。

### 3.4 引擎内部常量（对齐 JS native profile）

```rust
// layout.rs
const LINE_FIT_EPSILON:                      f32  = 0.005;
const CARRY_CJK_AFTER_CLOSING_QUOTE:         bool = false;
const PREFER_PREFIX_WIDTHS_FOR_BREAKABLE_RUNS: bool = true;
const PREFER_EARLY_SOFT_HYPHEN_BREAK:         bool = false;
const TAB_SIZE_PRE_WRAP:                      u8   = 8;  // 对齐 JS 默认，非 CSS 默认 4
```

---

## 四、Shaping 层（measure.rs）

### 4.1 ShapeCacheKey

```rust
#[derive(Hash, PartialEq, Eq)]
struct ShapeCacheKey {
    text_hash:  u64,   // ahash(text bytes)
    font_id:    FontId,
    size_px_q:  u32,   // (size_px * 64.0).round() as u32
    weight:     u16,
    italic:     bool,
    direction:  BidiDirection,  // Ltr | Rtl
    script:     u32,   // rustybuzz::Script as u32，避免类型泄漏到公共接口
    // lang: Option<u32>  ← v2 加入（Serbian vs Russian Cyrillic 等场景）
}
```

> **不用 `Ord::max` 替代 match**：enum variant 顺序和语义耦合，未来加 variant 会静默出错，保留显式 match。

### 4.2 ShapeCache

```rust
const SHAPE_CACHE_CAPACITY: usize = 2048;

pub struct ShapeCache {
    inner: LruCache<ShapeCacheKey, Arc<[ShapedGlyph]>>,
}

// 写入时：
let glyphs: Arc<[ShapedGlyph]> = Arc::from(shaped_vec);  // 零拷贝 move
// ❌ 不要：Arc::new(vec.clone().into_boxed_slice())      // 多余 clone
```

### 4.3 shape_run

```rust
// direction 必须在 shaping 层传给 rustybuzz，不在 layout 层补传
// 错误做法：shape 用默认 LTR，layout 层再翻转字形顺序
pub fn shape_run(
    text:      &str,
    face:      &FontFace,
    script:    rustybuzz::Script,
    direction: BidiDirection,     // 来自 bidi.rs 分析结果，直接透传
    cache:     &mut ShapeCache,
) -> Arc<[ShapedGlyph]>
```

### 4.4 prefix_widths（f64 内部累加）

```rust
pub fn prefix_widths(&self, text: &str, style: &TextStyleSpec) -> Arc<[f32]> {
    let glyphs = self.shape_with_fallback(text, style);
    let mut acc: f64 = 0.0;   // f64 内部累加，避免长文本误差累积导致换行点漂移
    let mut widths = Vec::with_capacity(glyphs.len() + 1);
    widths.push(0.0f32);
    for g in &glyphs {
        acc += g.advance as f64;
        widths.push(acc as f32);
    }
    Arc::from(widths)   // 结果进 ShapeCache，key = (text_hash, style_hash)
}
```

---

## 五、字体 Fallback 链（font_catalog.rs）

### 5.1 捆绑字体清单

| 文件 | 覆盖范围 |
|---|---|
| `NotoSans-Regular.ttf` | 拉丁、希腊、西里尔、阿拉伯、天城文、希伯来 |
| `NotoSansCJK-Regular.ttc` | CJK 统一汉字、假名、谚文 |
| `NotoSansMyanmar-Regular.ttf` | 缅甸文 |
| `NotoEmoji-Regular.ttf` | Emoji（文本变体 U+FE0E） |
| `NotoSansMono-Regular.ttf` | 等宽，代码片段 |

加载顺序：捆绑字体先注册 → 系统字体后注册。捆绑字体在 demo 场景始终优先。

### 5.2 coverage_map（O(1) 字符查询）

```rust
pub struct FontCatalog {
    // 初始化时构建一次，运行时 O(1) 查询
    // 热点字符 cache：HashMap<char, FontId, AHasher>
    char_to_font: HashMap<char, FontId, AHasher>,
    faces: Vec<FontFace>,
}

impl FontCatalog {
    // 初始化：遍历所有 face 的 cmap，构建 char → 优先级最高 FontId 的映射
    pub fn build(db: &fontdb::Database) -> Self { ... }

    // O(1) 查询
    pub fn font_for_char(&self, ch: char) -> FontId { ... }

    // 覆盖 cluster 内所有 codepoint 的最佳 face
    pub fn face_for_cluster(&self, cluster: &str) -> &FontFace {
        for face in &self.faces {
            // 必须覆盖 cluster 内所有 codepoint，不能 covers_any
            // flag emoji（🇯🇵 = U+1F1EF + U+1F1F5）和 ZWJ 序列整体判断
            if cluster.chars().all(|ch| face.has_glyph(ch)) {
                return face;
            }
        }
        &self.faces[0]  // notdef 兜底，不 panic
    }
}
```

### 5.3 font-run 切分（script-run → font-run）

```rust
fn is_complex_script(script: Script) -> bool {
    matches!(script,
        Script::Arabic | Script::Hebrew |
        Script::Devanagari | Script::Bengali | Script::Gurmukhi |
        Script::Myanmar | Script::Khmer | Script::Thai
    )
}

pub fn split_into_font_runs(text: &str, catalog: &FontCatalog) -> Vec<FontRun> {
    let mut output = Vec::new();

    for script_run in split_by_script(text) {   // unicode-script crate
        if is_complex_script(script_run.script) {
            output.extend(split_complex_script_run(&script_run, catalog));
        } else {
            // Latin / CJK：按 coverage 合并连续同 face 字符
            output.extend(split_by_coverage(&script_run, catalog));
        }
    }
    output
}

fn split_complex_script_run(run: &ScriptRun, catalog: &FontCatalog) -> Vec<FontRun> {
    let face = catalog.best_face_for_run(run);

    if run.text.chars().all(|ch| face.has_glyph(ch)) {
        // 理想：整段一个 face，shaping 上下文完整（joining / ligature 不被切断）
        return vec![FontRun::whole(run, face)];
    }

    // 有缺字：按 grapheme cluster 分配 face，合并连续同 face 的 cluster
    // 注意：每段仍然整段 shape，切分只决定"哪些 cluster 用哪个 face"
    let mut runs: Vec<FontRun> = Vec::new();
    let mut cur_face = catalog.face_for_cluster(&run.text[..]);
    let mut cur_start = 0usize;

    for (i, cluster) in run.text.grapheme_indices(true) {
        let best = catalog.face_for_cluster(cluster);
        if best.id() != cur_face.id() {
            runs.push(FontRun::new(&run.text[cur_start..i], cur_face));
            cur_face = best;
            cur_start = i;
        }
    }
    runs.push(FontRun::new(&run.text[cur_start..], cur_face));
    runs
}
```

---

## 六、Bidi 处理（bidi.rs）

```rust
pub struct BidiRun {
    pub byte_range: Range<usize>,
    pub level:      unicode_bidi::Level,
    pub direction:  BidiDirection,  // Ltr | Rtl
}

pub fn paragraph_to_bidi_runs(text: &str) -> Vec<BidiRun> {
    let bidi = BidiInfo::new(text, None);
    let para = &bidi.paragraphs[0];
    // 将 bidi.levels 数组中连续相同 level 的字节范围合并为一个 BidiRun
    // levels 是字节索引对应，需要按字符边界对齐
    coalesce_byte_levels(&bidi.levels, text)
}

// shaping：按逻辑顺序喂给 rustybuzz（每个 BidiRun 独立 shape）
// direction 在 shape_run() 里直接传给 rustybuzz::UnicodeBuffer::set_direction
// 视觉重排：在 layout_with_lines 输出之后，按 level 反转 RTL run 内字形顺序
// 混合方向行：LayoutLine 内保存 Vec<RunRef>，Painter 按视觉顺序绘制
```

---

## 七、Line Breaking（line_break.rs）

### 7.1 BreakOpportunity + merge

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BreakOpportunity {
    Allowed,
    Prohibited,
    Forced,
}

// 优先级：Forced > Prohibited > Allowed
// Forced 和 Prohibited 冲突时 Forced 赢（\n 强制换行不被 NBSP 禁止）
// ⚠️ 不用 Ord::max 替代：enum 顺序和语义耦合，未来加 variant 会静默出错
fn merge(a: BreakOpportunity, b: BreakOpportunity) -> BreakOpportunity {
    use BreakOpportunity::*;
    match (a, b) {
        (Forced, _) | (_, Forced)         => Forced,
        (Prohibited, _) | (_, Prohibited) => Prohibited,
        _                                 => Allowed,
    }
}
```

### 7.2 规则执行模型

```rust
// UAX14 是 baseline，上层规则只做 modify / suppress / force
// 不是"最后规则兜底"，而是"基础 + override"
pub fn break_opportunity(ctx: &BreakContext) -> BreakOpportunity {
    let mut b = uax14_baseline(ctx);     // unicode-linebreak crate

    for rule in OVERRIDE_RULES {
        b = merge(b, rule(ctx));
    }
    b
}

// 每个 override rule 签名：(ctx) -> BreakOpportunity
// 只处理自己关心的字符/上下文，其余返回 Allowed（由 merge 和 baseline 决定）
// 规则顺序不影响正确性（merge 是交换律的，Forced/Prohibited 优先级固定）
type BreakRule = fn(&BreakContext) -> BreakOpportunity;

const OVERRIDE_RULES: &[BreakRule] = &[
    rule_nbsp,           // U+00A0 禁止断行
    rule_wj,             // U+2060 禁止断行
    rule_zwsp,           // U+200B 允许断行
    rule_soft_hyphen,    // U+00AD 软连字符
    rule_cjk_punctuation,// CJK 句末标点不出现在行首
    rule_url_atom,       // URL 整体不断行
];
```

---

## 八、Layout 层（layout.rs）

### 8.1 四接口对等性保证

```rust
// 相同输入，四种 API 必须产生相同行数和行内容
// 测试用例：
let r1 = engine.layout(&p, w, lh).line_count;
let r2 = engine.layout_with_lines(&ps, w, lh).line_count;
let mut r3 = 0;
engine.walk_line_ranges(&ps, w, |_| r3 += 1);
let mut r4 = 0;
let mut cur = LayoutCursor { segment_index: 0, grapheme_index: 0 };
while engine.layout_next_line(&ps, &mut cur, w).is_some() { r4 += 1; }
assert_eq!((r1, r2, r3, r4), (r1, r1, r1, r1));
```

### 8.2 ParagraphCacheKey（P2 加入，方案先定结构）

```rust
#[derive(Hash, PartialEq, Eq)]
struct ParagraphCacheKey {
    text_hash:      u64,
    style_hash:     u64,
    width_bucket:   u32,         // quantize_width(w) = (w / 2.0).round() as u32
                                 // 2px bucket 吸收 egui resize 时的 1px 抖动
    obstacles_hash: u64,
    locale_hash:    u64,         // hash(locale_string)，None → 0
    white_space:    WhiteSpaceMode,
    // locale 和 white_space 影响 segmentation，不进 key 会导致 cache 命中错误
}

// 统一入口，全局只有这一处
#[inline]
fn quantize_width(w: f32) -> u32 {
    (w / 2.0).round() as u32
}
```

---

## 九、Demo App 架构

### 9.1 DemoWindow trait

```rust
// demos/mod.rs
pub trait DemoWindow {
    fn title(&self) -> &str;
    fn is_open(&self) -> bool;
    fn set_open(&mut self, open: bool);
    fn show(
        &mut self,
        ctx:    &egui::Context,
        engine: &PretextEngine,
        assets: &mut AssetRegistry,
    );
}
```

### 9.2 PretextDemoApp

```rust
// app.rs
pub struct PretextDemoApp {
    engine: PretextEngine,
    assets: AssetRegistry,
    demos:  Vec<Box<dyn DemoWindow>>,
}

impl eframe::App for PretextDemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for demo in &mut self.demos {
            if demo.is_open() {
                demo.show(ctx, &self.engine, &mut self.assets);
            }
        }
    }
}
```

### 9.3 脏标记与 egui 浮点抖动

```rust
struct DemoLayoutState {
    cache:          Option<Vec<LayoutLine>>,
    dirty:          bool,
    last_rect:      egui::Rect,
}

// rect 比较用 DPI 感知的 epsilon，避免每帧抖动触发 reflow
fn rect_needs_reflow(a: Rect, b: Rect, ppp: f32) -> bool {
    let eps = 1.0 / ppp;   // ppp = ctx.pixels_per_point()
    (a.width()  - b.width()).abs()  > eps ||
    (a.height() - b.height()).abs() > eps
}

// show() 内：
let available = ui.available_rect_before_wrap();
if state.dirty || rect_needs_reflow(available, state.last_rect, ctx.pixels_per_point()) {
    state.cache = Some(reflow(&engine, available));
    state.dirty = false;
    state.last_rect = available;
}
// 直接用 cache 绘制，不重算
```

### 9.4 SVG Texture 生命周期

```rust
// assets.rs
pub struct AssetRegistry {
    textures: HashMap<String, egui::TextureHandle>,
    // TextureHandle 内部是 Arc，drop 时引用计数归零，egui 自动释放 GPU 资源
}

impl AssetRegistry {
    /// SVG → resvg Pixmap → image::RgbaImage → egui ColorImage → TextureHandle
    /// 首次调用时栅格化并上传，后续返回已有 handle（常驻内存，不随窗口开关重复上传）
    pub fn get_or_load_svg(
        &mut self, key: &str, svg_bytes: &[u8],
        size: [usize; 2], ctx: &egui::Context,
    ) -> &egui::TextureHandle { ... }
}
```

### 9.5 各 Demo 实现要点

**catalog**：列出所有 demo，点击切换 `is_open()`，始终显示不可关闭。

**accordion**
```
4 个 section，one-open-at-a-time
折叠高度 = 固定 header（约 40px）
展开高度 = engine.layout(section_text, ...).height + padding
动画：egui lerp 插值，每帧 ctx.request_repaint()
```

**bubbles**
```
左侧（CSS fit-content 等效）：layout(text, container_width * 0.8, lh)，取最大行宽
右侧（shrinkwrap）：二分搜索 walk_line_ranges()，找最小 width 使 line_count 不变
slider 控制 container_width（100–500px）
wasted_area = (standard_width - shrinkwrap_width) * line_count * line_height
```

**rich_note**
```
片段：Text | Code | Link | Chip（Chip = AtomicPlaceholder，不可断）
Chip 在 prepare_with_segments 之前替换为 AtomicPlaceholder（固定宽度，不可断）
用 layout_next_line() 逐行流式排布
Chip 渲染为带背景色圆角矩形
```

**masonry**
```
数据：assets/masonry.json，≤200 张卡片
列数：floor(available_width / (card_min_width + gap))
高度：engine.layout(card_text, col_width, lh).height + padding
贪心放置：每次选最短列
memo_key = hash(cards + col_width)，相同则跳过重算
可见性裁剪：ScrollArea::vertical().show_viewport + allocate_rect 占位
```

**dynamic_layout**
```
SVG logo → AssetRegistry.get_or_load_svg → egui::Image
alpha-hull：遍历 Pixmap 像素，提取非透明边界点，礼品包裹法求凸包
点击 logo：启动旋转动画（角度每帧增量，ctx.request_repaint）
文字绕排：logo hull AABB 作为障碍物，传入 layout_next_line 自定义宽度
```

**editorial_engine**
```
orb：Vec<Orb { pos, vel, radius }>，每帧物理更新（paused 时跳过）
障碍物：orb 生成圆形遮挡区间
drop cap：首字放大（size_px * 3），占前 3 行左侧，固定 Obstacle
pull quote：右侧列独立 layout，其 rect 作为 Obstacle
文字流：三列，各列独立 layout_next_line 游标
reflow 触发：orb 越过行边界（AABB 粗判）|| 窗口 resize
动画与 reflow 解耦：位移每帧更新，reflow 只在边界事件触发
```

**variable_typographic_ascii**
```
粒子场：Vec<Particle { char, x, y, vx, vy }>
字符宽度：engine.glyph_advance(ch, &proportional_style)
等宽对比：左列 NotoSansMono，右列 NotoSans，并排显示
per-glyph 着色：按速度映射颜色，Painter::text 逐字符绘制
```

---

## 十、JS → Rust 移植陷阱

逐函数移植，每函数移植后立即补测试，禁止批量移植后补测试。

| JS 模式 | Rust 处理 |
|---|---|
| `arr[i]` 越界 → `undefined` | `arr.get(i)` → `Option`，显式处理 `None` |
| `x \|\| fallback` | `Option::unwrap_or(fallback)` |
| `NaN` 静默传播 | 所有 `f32` 运算后加 `debug_assert!(!v.is_nan())` |
| 对象属性缺失 → `undefined` | Rust struct 全字段显式初始化 |
| `typeof x === 'string'` 分支 | Rust enum variant，编译期穷举 |
| 浮点 `===` 比较 | `(a - b).abs() < LINE_FIT_EPSILON` |
| `Math.min/max` 链 | `f32::min/max`，注意 `NaN` 传播行为不同 |
| 闭包捕获外部可变变量 | 借用规则，必要时用 `Cell<T>` |
| `Map<string, T>` | `HashMap<String, T, AHasher>` |

---

## 十一、测试计划

### 11.1 引擎单元测试

**Shaping（Step 1 完成后锁定）**
- Latin 正向：`"hello"` shape 产生正确 advance
- Arabic：liga feature 生效，run 方向 RTL
- Emoji ZWJ 序列：👨‍👩‍👧‍👦 作为单个不可分 cluster

**Font Fallback（Step 2 完成后锁定）**
- Latin + emoji 混排：两段各用对应 face
- Arabic + emoji：Arabic 整段一个 face，emoji 切换 face
- 全部 face 无法覆盖时：插入 notdef，不 panic
- `face_for_cluster` 使用 covers_all，不是 covers_any

**空白处理**
- `Normal` 模式：连续空白折叠为单空格，行首尾裁剪
- `PreWrap` 模式：空白保留，`\t` 按 tab_size=8 展开，`\n` 强制断行
- NBSP（U+00A0）不折叠、不断行

**Line Break（Step 3 完成后锁定）**
- NBSP → Prohibited（不断行）
- ZWSP → Allowed（可断行，零宽）
- `\n` PreWrap → Forced（强制换行）
- Forced + Prohibited 同时：Forced 赢
- URL 整体不断行
- CJK 句末标点不出现在行首
- 软连字符：仅在实际断行处显示，否则不占宽度

**四接口对等（Step 4 完成后锁定）**
- `layout` / `layout_with_lines` / `walk_line_ranges` / `layout_next_line` 在 12 个场景下行数和行内容完全一致

**prefix_widths**
- 数组长度 == grapheme cluster 数
- `widths[0] == 0.0`
- 等宽字体：所有相邻差值相等
- 长文本（1000 字）：末尾累加误差 < 0.1px

### 11.2 Demo 逻辑测试

| Demo | 测试点 |
|---|---|
| bubbles | shrinkwrap 后 line_count 与 normal 相同；shrinkwrap_width ≤ normal_width |
| rich_note | 每个 Chip 的 byte range 不被任何 LayoutLine 跨越切分 |
| dynamic_layout | alpha-hull polygon 点数 > 0；hull 包含原始 logo 矩形四角 |
| editorial_engine | 障碍物加入后总行数增加；相同障碍物多次 reflow 结果完全一致 |
| masonry | 相同输入多次调用 layout_masonry 输出完全相同（无随机性） |
| accordion | 展开高度 == `engine.layout(...).height + padding` |

### 11.3 Golden 回归测试

`layout_with_lines` 输出序列化为 `tests/goldens/*.json`，CI 中对比。

覆盖 12 个场景：
1. 纯英文，Normal
2. 纯英文，PreWrap（含 \t \n）
3. 纯阿拉伯文，RTL
4. 纯 CJK
5. 缅甸文
6. 混合方向（英文 + 阿拉伯）
7. 含 NBSP / WJ / ZWSP
8. 含软连字符
9. 含 Emoji ZWJ 序列
10. 含 URL
11. 数字 + 标点混排
12. 含障碍物（editorial_engine 场景）

### 11.4 无头冒烟测试

```rust
#[test]
fn all_demos_open_without_panic() {
    let ctx = egui::Context::default();
    let mut app = PretextDemoApp::new_headless();
    for demo in app.demos_mut() { demo.set_open(true); }
    ctx.run(egui::RawInput::default(), |ctx| app.update_headless(ctx));
    // 不 panic 即通过
}
```

---

## 十二、实现顺序

按依赖关系从底层到上层，每步完成后跑对应测试锁定：

```
Step 1  shaping + ShapeCache
        rustybuzz shape_run，key 含 font_id / direction / script
        测试：Latin / Arabic / emoji ZWJ

Step 2  font fallback（split_into_font_runs）
        coverage_map O(1) 查询
        complex script 按 grapheme cluster 切
        测试：Latin+emoji / Arabic+emoji / CJK 混排

Step 3  line_break（UAX14 baseline + override + merge）
        测试：NBSP / ZWSP / Forced+Prohibited / URL / CJK 标点

Step 4  layout（greedy，四接口，prefix_widths）
        测试：四接口对等，prefix_widths 精度，12 golden 锁定

Step 5  demo app 骨架 + catalog + accordion + bubbles
        无头冒烟测试通过

Step 6  rich_note + masonry + variable_typographic_ascii
        对应 demo 逻辑测试通过

Step 7  dynamic_layout + editorial_engine（障碍物 reflow，脏标记）
        60fps 目标验证

Step 8  SVG 纹理 + alpha-hull + golden 回归 CI + README
        CI 全绿
```

---

## 十三、阶段划分

| 阶段 | 内容 | 完成标志 |
|---|---|---|
| **P0** | Workspace 骨架、字体加载、fontdb + coverage_map | `cargo test font` 全绿 |
| **P1** | shaping、bidi、line_break、layout 四接口 | 12 golden 锁定，四接口对等测试通过 |
| **P2** | demo 骨架、catalog、accordion、bubbles、rich_note | 无头冒烟通过；ParagraphCache 加入 |
| **P3** | masonry、variable_typographic_ascii | 对应 demo 逻辑测试通过 |
| **P4** | dynamic_layout、editorial_engine | 脏标记验证，60fps |
| **P5** | SVG 纹理、alpha-hull、CI、README | CI 全绿 |

---

## 十四、v1 范围外的已知遗留项

| 项 | 说明 |
|---|---|
| ShapeCacheKey 加 `lang` | Serbian vs Russian Cyrillic 等 language-specific shaping；v1 只有 `script` |
| ParagraphCache 实现 | 结构已在方案中定义（§8.2），P2 实现 |
| InlineItem 完整模型 | P2/P3，v1 用 `AtomicPlaceholder` 预留结构位 |
| Bidi cursor / hit-test | `hit_test(x,y)` / `position_to_xy`；需要才做，不在 demo 目标内 |
| 浏览器对标测试 | headless Chrome + Playwright 对比行数；v2 基础设施 |
| WASM 目标 | eframe 支持；届时 fontdb 只用捆绑字体，fallback 链逻辑不变 |
| Accessibility | egui accesskit 在 0.33 是 beta |
| RTL UI 镜像 | demo 窗口 UI 方向不做 RTL 镜像，只保证 RTL 文本内容正确 |
| Variable Font | rustybuzz 支持 variation axes；demo 字体集为 static，用固定字重模拟 |
| 子像素渲染 | egui 整像素光栅化，无 ClearType，与浏览器有视觉差异，属预期偏差 |
| `system-ui` 字体 | JS 文档明确标注 macOS 下测量不准，Rust 端同样只用具名字体 |
