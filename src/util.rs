//! Utilities for the standard rules.

use std::{io::Read, pin::Pin};

use anyhow::anyhow;
use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, XzDecoder, ZstdDecoder};
use futures::stream::Stream;
use pb_rules_sdk::{futures::FutureCompat2, pb::rules::write_filesystem::WriteDirectory};
use smallvec::SmallVec;
use tar::EntryType;

/// Wraps the provided `incoming` stream in types to decompress the underlying data.
pub fn decompress_stream(
    incoming: impl Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send + 'static,
    content_encoding: Option<&str>,
    content_disposition: Option<&str>,
    _filename: Option<&str>,
) -> Result<impl Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send + 'static, String> {
    let encodings = match content_encoding.as_ref() {
        Some(header) => header
            .split(',')
            .map(|component| {
                Compression::from_content_encoding(component)
                    .ok_or_else(|| format!("unrecognized component: '{component}'"))
            })
            .collect::<Result<SmallVec<[Compression; 2]>, _>>()?,
        None => SmallVec::default(),
    };

    let disposition =
        content_disposition.map(|x| content_disposition::parse_content_disposition(x));

    // TODO: A better merge.
    let algos: Box<dyn Iterator<Item = Compression>> = if !encodings.is_empty() {
        Box::new(encodings.iter().rev().copied())
    } else if disposition.is_some() {
        let disposition = disposition.expect("checked above");
        match disposition.filename_full() {
            Some(filename) => Box::new(Compression::from_filename(&filename).into_iter()),
            None => Box::new(std::iter::empty()),
        }
    } else {
        Box::new(std::iter::empty())
    };

    // TODO: Also check the filename here.

    let mut reader: Pin<Box<dyn tokio::io::AsyncBufRead + Send + 'static>> =
        Box::pin(tokio_util::io::StreamReader::new(incoming));

    // Apply all of the compression algorithms.
    for compression in algos {
        // There is a lot of boxing going on here, but seems unavoidable given how the
        // various types are structured in `async-compression` and `tokio`.
        let x: Pin<Box<dyn tokio::io::AsyncRead + Send + 'static>> = match compression {
            Compression::Brotli => Box::pin(BrotliDecoder::new(reader)),
            Compression::Gzip => Box::pin(GzipDecoder::new(reader)),
            Compression::Xz => Box::pin(XzDecoder::new(reader)),
            Compression::Zstd => Box::pin(ZstdDecoder::new(reader)),
        };
        reader = Box::pin(tokio::io::BufReader::new(x));
    }

    Ok(tokio_util::io::ReaderStream::new(reader))
}

/// Compression formats that we support.
#[derive(Copy, Clone, Debug)]
enum Compression {
    Brotli,
    Gzip,
    Xz,
    Zstd,
}

impl Compression {
    pub fn from_content_encoding(component: &str) -> Option<Self> {
        match component {
            "br" => Some(Compression::Brotli),
            "gzip" => Some(Compression::Gzip),
            "xz" => Some(Compression::Xz),
            "zstd" => Some(Compression::Zstd),
            _ => None,
        }
    }

    pub fn from_filename(filename: &str) -> Option<Self> {
        // TODO: fillout
        if filename.ends_with("zst") || filename.ends_with("zstd") {
            Some(Compression::Zstd)
        } else {
            None
        }
    }
}

/// Given a directory to write into, reconstruct a [`tar`] archive.
pub async fn reconstruct_tar(
    directory: WriteDirectory,
    data: impl std::io::Read,
) -> Result<WriteDirectory, anyhow::Error> {
    let mut archive = tar::Archive::new(data);
    let entries = archive.entries()?;

    for entry in entries {
        let mut entry = entry?;
        let path = entry.path()?;
        let path = path.to_str().ok_or_else(|| anyhow!("non UTF-8 path"))?;

        match entry.header().entry_type() {
            EntryType::Directory => {
                tracing::trace!(?path, "creating directory");
                let directory = directory
                    .create_directory(path)
                    .compat()
                    .await
                    .map_err(|err| anyhow!("{err}"))?;
                directory
                    .close()
                    .compat()
                    .await
                    .map_err(|err| anyhow!("{err}"))?;
            }
            EntryType::Regular => {
                tracing::trace!(?path, "creating file");

                let file = directory
                    .create_file(path)
                    .compat()
                    .await
                    .map_err(|err| anyhow!("{err}"))?;

                let mut buf = vec![0u8; 4096];
                loop {
                    let bytes_read = entry.read(&mut buf[..])?;

                    // We're done!
                    if bytes_read == 0 {
                        file.close()
                            .compat()
                            .await
                            .map_err(|err| anyhow!("{err}"))?;
                        break;
                    }

                    // Write into the repository.
                    file.append(&buf[..bytes_read])
                        .compat()
                        .await
                        .map_err(|err| anyhow!("{err}"))?;
                }
            }
            other => {
                tracing::info!(?other, "skipping unsupported type");
            }
        }

        tracing::info!(path = ?entry.path(), "got entry");
    }

    Ok(directory)
}
