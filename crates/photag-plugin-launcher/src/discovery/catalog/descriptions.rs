use std::path::Path;
pub(crate) fn program_description(name: &str, _path: &Path, category: &str) -> String {
    let name = name.to_lowercase();
    if name.trim() == "ima" {
        return "腾讯 AI 知识助手，用于整理资料、构建知识库并基于资料问答。".to_owned();
    }
    let known = [
        (
            &["anchorpoint"][..],
            "基于 Git 管理创作项目的版本与素材，支持文件锁定和团队协作。",
        ),
        (&["clustta"], "用于管理创意项目文件、素材版本和团队协作。"),
        (&["reasonix"], "面向终端、桌面和代码编辑器的 AI 编程助手。"),
        (
            &["cc switch"],
            "用于切换和管理 AI 编程工具的模型及服务配置。",
        ),
        (
            &["skills manager"],
            "用于搜索、安装和统一管理多个 AI 编程助手使用的技能包。",
        ),
        (
            &["skills-manager"],
            "用于搜索、安装和统一管理多个 AI 编程助手使用的技能包。",
        ),
        (
            &["qoder"],
            "面向软件开发的 AI IDE，可理解代码库并协助编写、修改和调试代码。",
        ),
        (
            &["deepseek"],
            "DeepSeek AI 助手，用于对话问答、学习和内容创作。",
        ),
        (
            &["workbuddy"],
            "WorkBuddy 是 AI 助手，用于辅助处理资料、任务和日常工作。",
        ),
        (&["affine"], "用于创建文档、白板和知识库的协作工作空间。"),
        (&["anytype"], "用于本地优先地组织笔记、任务和个人知识。"),
        (&["allusion"], "用于收集、整理和浏览视觉参考图片。"),
        (&["eagle"], "用于收集、分类和检索图片及其他灵感素材。"),
        (
            &["cpu-z", "cpuid"],
            "用于查看处理器、主板、内存和显卡的硬件信息。",
        ),
        (&["fancontrol"], "用于监测温度并配置电脑风扇转速曲线。"),
        (
            &["internet download manager"],
            "用于管理和加速文件下载任务。",
        ),
        (
            &["imagemagick", "image magick"],
            "用于转换、编辑和处理多种格式的图像文件。",
        ),
        (&["notepad2"], "用于查看和编辑文本、脚本及源代码文件。"),
        (&["ride"], "用于编写、运行和调试软件代码的集成开发环境。"),
        (&["彩读"], "用于阅读本地 TXT 小说并为文本内容添加颜色。"),
        (&["uu远程"], "用于远程连接和控制其他设备。"),
        (&["momask"], "用于通过生成式模型创建三维人体动作内容。"),
        (&["一修大师"], "用于管理和启动游戏修改工具。"),
        (&["flingtrainer"], "用于为单机游戏启用和管理训练器功能。"),
        (
            &["penguincanvas", "无限画布"],
            "用于在画布上协作生成和整理图像、视频等创作内容。",
        ),
        (
            &["ztools"],
            "通过快捷键呼出的插件化效率工具箱，可启动应用并使用扩展工具。",
        ),
        (&["localsend"], "用于在同一网络中的设备间直接传送文件。"),
        (&["photoshop"], "用于图像处理、照片修饰、合成与视觉设计。"),
        (
            &["visual studio code", "vs code"],
            "用于编写、调试和管理代码项目的轻量代码编辑器。",
        ),
        (
            &["visual studio"],
            "用于开发、构建和调试 Windows 及跨平台软件的集成开发环境。",
        ),
        (&["affinity"], "用于图像编辑、绘图和版面设计的创意软件。"),
        (&["autocad"], "用于创建和编辑二维工程图及三维 CAD 设计。"),
        (&["cadcraft"], "用于绘制和编辑二维工程图及三维 CAD 设计。"),
        (
            &["lightcraft"],
            "用于管理照片图库并进行 RAW 图像编辑与非破坏性调色。",
        ),
        (
            &["neatreader"],
            "用于阅读和管理 EPUB 电子书，并在设备间同步阅读进度。",
        ),
        (
            &["blender"],
            "用于三维建模、动画、渲染和视频编辑的创作工具。",
        ),
        (&["krita"], "用于数字绘画与插画创作。"),
        (
            &["7-zip", "7zip"],
            "用于压缩、解压和管理常见格式的归档文件。",
        ),
        (&["chrome"], "用于浏览网页和运行 Web 应用。"),
        (&["edge"], "用于浏览网页和运行 Web 应用。"),
        (
            &["wechat", "weixin", "微信"],
            "用于即时通信、群聊、语音视频通话和文件传递。",
        ),
        (&["telegram"], "用于即时通信、群组交流和文件分享。"),
        (&["discord"], "用于社区文字、语音和视频交流。"),
        (&["steam"], "用于购买、管理和启动 PC 游戏及相关社区功能。"),
        (&["everything"], "通过文件名快速搜索本机文件和文件夹。"),
        (
            &["quicklook"],
            "通过空格键快速预览文件，无需先打开对应应用。",
        ),
        (&["onecommander"], "用于浏览、整理和管理本地文件与文件夹。"),
        (&["fastcopy"], "用于高速复制、移动和校验文件。"),
        (
            &["obsidian"],
            "用于创建互相关联的 Markdown 笔记和个人知识库。",
        ),
        (&["notion"], "用于管理笔记、文档、任务和协作工作区。"),
        (&["wps"], "用于创建和编辑文字文档、电子表格及演示文稿。"),
        (&["potplayer"], "用于播放多种格式的本地音频和视频。"),
        (&["foobar2000"], "用于播放和管理数字音乐。"),
        (
            &["powershell"],
            "用于运行命令、自动化脚本和管理 Windows 系统。",
        ),
        (
            &["putty", "xshell"],
            "用于通过 SSH、Telnet 等协议连接和管理远程主机。",
        ),
        (&["todesk"], "用于远程访问和控制其他设备。"),
        (&["moonlight"], "用于将游戏或桌面从远程设备串流到本机。"),
        (&["sunshine"], "用于向 Moonlight 客户端串流本机游戏或桌面。"),
        (
            &["synology drive"],
            "用于在本机与 Synology NAS 之间同步和备份文件。",
        ),
        (&["onedrive"], "用于同步本机文件与 Microsoft 云端存储。"),
        (
            &["百度网盘"],
            "用于管理百度网盘中的文件并进行上传、下载和同步。",
        ),
        (&["坚果云"], "用于在多台设备间同步和共享文件。"),
        (&["夸克网盘"], "用于管理夸克网盘中的文件并进行上传和下载。"),
        (&["excel"], "用于创建电子表格、计算数据和制作图表。"),
        (&["powerpoint"], "用于制作和演示幻灯片。"),
        (&["word"], "用于创建和编辑文字文档。"),
        (&["onenote"], "用于记录、整理和检索笔记。"),
    ];
    if let Some((_, description)) = known
        .iter()
        .find(|(terms, _)| terms.iter().any(|term| name.contains(term)))
    {
        return (*description).to_owned();
    }
    let purpose = match category {
        "AI 工具" => "运行人工智能对话、生成或辅助创作任务",
        "文件与同步" => "浏览、搜索、传输或同步文件",
        "远程连接" => "连接、串流或远程控制其他设备",
        "开发工具" => "编写、调试、构建或管理软件项目",
        "设计创作" => "创作或编辑图像、视频及三维内容",
        "办公软件" => "创建、编辑和管理文档与工作资料",
        "通讯社交" => "进行即时通信和在线协作",
        "网络浏览" => "访问网页或管理网络连接",
        "影音播放" => "播放或处理音频与视频",
        "游戏" => "启动或管理游戏及相关服务",
        "系统工具" => "管理 Windows、硬件或系统设置",
        _ => "启动该程序并使用其提供的功能",
    };
    format!("用于{purpose}。")
}

pub(crate) fn has_product_description(name: &str) -> bool {
    let name = name.to_lowercase();
    if name.trim() == "ima" {
        return true;
    }
    [
        "anchorpoint",
        "clustta",
        "reasonix",
        "cc switch",
        "skills manager",
        "skills-manager",
        "eagle",
        "cpu-z",
        "cpuid",
        "fancontrol",
        "internet download manager",
        "imagemagick",
        "notepad2",
        "ride",
        "彩读",
        "uu远程",
        "penguincanvas",
        "无限画布",
        "ztools",
        "momask",
        "一修大师",
        "flingtrainer",
        "qoder",
        "deepseek",
        "workbuddy",
        "affine",
        "anytype",
        "allusion",
        "localsend",
        "photoshop",
        "visual studio code",
        "vs code",
        "visual studio",
        "affinity",
        "autocad",
        "cadcraft",
        "lightcraft",
        "neatreader",
        "blender",
        "krita",
        "7-zip",
        "7zip",
        "chrome",
        "edge",
        "wechat",
        "weixin",
        "微信",
        "telegram",
        "discord",
        "steam",
        "everything",
        "quicklook",
        "onecommander",
        "fastcopy",
        "obsidian",
        "notion",
        "wps",
        "potplayer",
        "foobar2000",
        "powershell",
        "putty",
        "xshell",
        "todesk",
        "moonlight",
        "sunshine",
        "synology drive",
        "onedrive",
        "百度网盘",
        "坚果云",
        "夸克网盘",
    ]
    .iter()
    .any(|term| name.contains(term))
}

pub(crate) fn clean_registry_description(value: &str) -> Option<String> {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.chars().count() < 16
        || value.contains('@')
        || value.to_lowercase().contains("installer")
    {
        return None;
    }
    let end = value
        .char_indices()
        .find(|(index, ch)| {
            matches!(ch, '。' | '！' | '？')
                || (matches!(ch, '.' | '!' | '?')
                    && value[*index..]
                        .chars()
                        .nth(1)
                        .is_some_and(|next| next.is_whitespace()))
        })
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(value.len());
    let sentence = value[..end].trim();
    if sentence.is_empty() {
        return None;
    }
    Some(
        if sentence
            .chars()
            .last()
            .is_some_and(|ch| matches!(ch, '。' | '！' | '？' | '.' | '!' | '?'))
        {
            sentence.to_owned()
        } else {
            format!("{sentence}。")
        },
    )
}
