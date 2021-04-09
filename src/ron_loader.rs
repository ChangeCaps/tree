#[macro_export]
macro_rules! ron_loader {
    ($loader:path, $($extension:expr => $asset:path),+) => {
        impl bevy::asset::AssetLoader for $loader
        {
            fn load<'a>(
                &'a self,
                bytes: &'a [u8],
                load_context: &'a mut bevy::asset::LoadContext,
            ) -> bevy::utils::BoxedFuture<'a, Result<(), anyhow::Error>> {
                Box::pin(async move {
                    match load_context.path().extension().unwrap().to_str().unwrap() {
                        $(
                            $extension => {
                                let asset = ron::de::from_bytes::<$asset>(bytes)
                                    .map_err(|e| {
                                        anyhow::Error::msg(
                                            format!("'{}': {}", load_context.path().to_string_lossy(), e)
                                        )
                                    })?;
                                load_context.set_default_asset(bevy::asset::LoadedAsset::new(asset));
                            },
                        )+
                        e => unreachable!("{:?}", e),
                    }

                    Ok(())
                })
            }

            fn extensions(&self) -> &[&str] {
                &[
                    $($extension,)+
                ]
            }
        }
    };
}
