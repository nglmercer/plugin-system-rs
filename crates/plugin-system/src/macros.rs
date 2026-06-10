#[macro_export]
macro_rules! plugin_metadata {
    (
        name: $name:expr,
        version: $version:expr,
        authors: [$($author:expr),* $(,)?],
        dependencies: [$($dep:expr),* $(,)?]
    ) => {
        $crate::traits::PluginMetadata {
            name: $name.to_string(),
            version: $version.to_string(),
            authors: vec![$($author.to_string()),*],
            dependencies: vec![$($dep.to_string()),*],
        }
    };
}
