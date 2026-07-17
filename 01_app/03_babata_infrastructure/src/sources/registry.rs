use babata_domain::SourceRouteDescriptor;

use super::providers;

pub fn descriptors() -> Vec<SourceRouteDescriptor> {
    vec![
        providers::feishu::descriptor(),
        providers::yuque::descriptor(),
        providers::onenote::descriptor(),
        providers::evernote::descriptor(),
        providers::wechat::descriptor(),
        providers::zhihu::descriptor(),
        providers::bilibili::descriptor(),
        providers::xiaohongshu::descriptor(),
        providers::douyin::descriptor(),
        providers::browser::descriptor(),
        providers::conversations::descriptor(),
        providers::local_files::descriptor(),
        providers::first_party::descriptor(),
    ]
}
