#[macro_export]
macro_rules! plugin_metadata {
    (
        name: $name:expr,
        version: $version:expr,
        authors: [$($author:expr),* $(,)?],
        dependencies: [$(dep($dname:expr, $dreq:expr)),* $(,)?]
    ) => {
        $crate::traits::PluginMetadata {
            name: $name.to_string(),
            version: $version.to_string(),
            authors: vec![$($author.to_string()),*],
            dependencies: vec![$($crate::traits::PluginDependency {
                name: $dname.to_string(),
                version_req: $dreq.to_string(),
            }),*],
        }
    };
}

#[macro_export]
macro_rules! dep {
    ($name:expr, $version_req:expr) => {
        $crate::traits::PluginDependency {
            name: $name.to_string(),
            version_req: $version_req.to_string(),
        }
    };
}
