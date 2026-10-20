use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use tracing::{debug, error, instrument, trace, warn};

use crate::error::{AcfReadError, Result, StreamError};
use crate::parse::AcfTokenStream;
use crate::stream::{AcfHashMap, AcfHashMapValue, StructuredAcfTokenStream};
use crate::token::AcfToken;

#[instrument(level = "debug", ret)]
fn select_next_in_stream<'a, R, T, U>(
    stream: &mut StructuredAcfTokenStream<R>,
    selector: &'a T,
) -> Result<AcfToken>
where
    R: Read + Debug,
    &'a T: IntoIterator<Item = U> + Debug,
    U: AsRef<str>,
{
    stream.select_path(selector)?;
    stream.expect_next()
}

#[instrument(level = "debug", ret)]
fn stream_from_file<P>(file_path: P) -> Result<StructuredAcfTokenStream<File>>
where
    P: AsRef<Path> + Debug,
{
    let file_path = file_path.as_ref();
    let acf_handle = File::open(file_path)
        .map_err(|e| {
            AcfReadError::from(e)
                .append_help(format!("ACF Path: {}", file_path.display()))
                .append_help("If the ACF file is missing, please ensure the game is installed and that the steamid is correct")
        })?;

    Ok(StructuredAcfTokenStream::new(AcfTokenStream::new(
        acf_handle,
    )))
}

#[instrument(level = "debug", ret)]
pub fn select_string_in_file<'a, P, T, U>(file: P, selector: &'a T) -> Result<String>
where
    P: AsRef<Path> + Debug,
    &'a T: IntoIterator<Item = U> + Debug,
    U: AsRef<str>,
{
    let mut stream = stream_from_file(file)?;
    let installdir = match select_next_in_stream(&mut stream, selector)? {
        AcfToken::String(dir) => Ok(dir),
        token => Err(StreamError::UnexpectedToken { token }),
    }?;

    Ok(installdir)
}

#[instrument(level = "debug", ret)]
pub fn game_paths(steam_root: &Path) -> Result<HashMap<u32, String>> {
    let library_config = steam_root.join("steam/config/libraryfolders.vdf");

    let mut stream = stream_from_file(library_config)?;

    let parsed_config: HashMap<String, AcfHashMapValue> = match select_next_in_stream(
        &mut stream,
        &["libraryfolders"],
    )? {
        AcfToken::DictStart => <&mut StructuredAcfTokenStream<File> as std::convert::TryInto<
            AcfHashMap,
        >>::try_into(&mut stream)?
        .into(),

        token => return Err(StreamError::UnexpectedToken { token }.into()),
    };

    let mut game_paths: HashMap<u32, String> = HashMap::new();

    for (library_idx, library_def) in parsed_config {
        let lib: HashMap<_, _> = match library_def {
            AcfHashMapValue::String(s) => {
                warn!(
                    "Failed to read library info for entry '{library_idx}': Expected a dict, but found the string '{s}'"
                );
                continue;
            }
            AcfHashMapValue::Hash(l) => l.into(),
        };

        let lib_path = match lib.get("path") {
            Some(AcfHashMapValue::String(str)) => str,
            Some(AcfHashMapValue::Hash(_)) => {
                warn!(
                    "Failed to read library path for entry '{library_idx}': Expected path to be a string, but found a dict"
                );
                continue;
            }
            None => {
                warn!(
                    "Failed to read library info for entry '{library_idx}': Expected a path, but found nothing"
                );
                continue;
            }
        };

        let lib_label = match lib.get("label") {
            Some(AcfHashMapValue::String(str)) => str,
            Some(AcfHashMapValue::Hash(_)) => {
                warn!(
                    "Failed to read library label for entry '{library_idx}': Expected label to be a string, but found a dict"
                );
                continue;
            }
            None => "[unnamed]",
        };

        debug!("Found a library: '{lib_label}' => {lib_path}");

        let apps = match lib.get("apps") {
            Some(AcfHashMapValue::Hash(apps)) => {
                debug!("Found {} games for '{lib_label}'", apps.len());
                trace!("{apps:?}");
                apps
            }
            Some(AcfHashMapValue::String(s)) => {
                warn!(
                    "Failed to read library apps for entry '{library_idx}': Expected a dict, but found the string '{s}'"
                );
                continue;
            }
            None => {
                warn!(
                    "Failed to read library apps for entry '{library_idx}': Expected key 'apps', but found nothing"
                );
                continue;
            }
        };

        for key in apps.keys() {
            let game_path = format!("{lib_path}/steamapps/appmanifest_{key}.acf");
            match key.parse() {
                Ok(id) => {
                    game_paths.insert(id, game_path);
                }
                Err(e) => error!("Invalid appid! {key}: {e}"),
            }
        }
    }

    Ok(game_paths)
}
