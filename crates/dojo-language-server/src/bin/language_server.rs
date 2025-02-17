use std::sync::Arc;

use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_language_server::Backend;
use cairo_lang_starknet::plugin::StarkNetPlugin;
use cairo_lang_utils::logging::init_logging;
use dojo_lang::plugin::DojoPlugin;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    init_logging(log::LevelFilter::Warn);

    #[cfg(feature = "runtime-agnostic")]
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    #[cfg(feature = "runtime-agnostic")]
    let (stdin, stdout) = (stdin.compat(), stdout.compat_write());

    let db = RootDatabase::builder()
        .with_cfg(CfgSet::from_iter([Cfg::name("test")]))
        .with_semantic_plugin(Arc::new(DojoPlugin))
        .with_semantic_plugin(Arc::new(StarkNetPlugin::default()))
        .build()
        .unwrap_or_else(|error| {
            panic!("Problem creating language database: {error:?}");
        });

    let (service, socket) = LspService::build(|client| Backend::new(client, db.into()))
        .custom_method("vfs/provide", Backend::vfs_provide)
        .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
