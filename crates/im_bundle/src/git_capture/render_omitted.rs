// Path: crates/im_bundle/src/git_capture/render_omitted.rs
// Description: Model-readable listing of changed paths the bundle selection omitted

use super::status::OmittedPath;

pub(crate) fn render_omitted_paths(omitted: Option<&[OmittedPath]>) -> Vec<u8> {
    let mut output = String::new();
    output.push_str("Intermediary changed paths omitted by bundle selection\n");
    output.push_str("=====================================================\n\n");
    output.push_str(
        "Each line is: <XY status>\\t<repository path>\\t<omission reason>.\nNames are disclosed so a reviewer can judge what the selection left out; the content of these paths is not in this bundle.\n\n",
    );
    match omitted {
        None => output.push_str("(unavailable: repository status could not be captured)\n"),
        Some([]) => output.push_str("(none: every changed path is inside the bundle selection)\n"),
        Some(paths) => {
            for entry in paths {
                output.push_str(&entry.xy);
                output.push('\t');
                output.push_str(&entry.path.display());
                output.push('\t');
                output.push_str(&entry.reason.to_string());
                output.push('\n');
            }
        }
    }
    output.into_bytes()
}
