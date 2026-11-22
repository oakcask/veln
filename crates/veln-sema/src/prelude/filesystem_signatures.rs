pub(super) fn filesystem_io_arity(name: &str) -> Option<usize> {
    match name {
        "io_provider_scope_owner" => Some(0),
        "fs_system_open"
        | "fs_read_only"
        | "io_provider_read_file"
        | "io_provider_write_file"
        | "io_provider_update_file"
        | "io_provider_append_file"
        | "io_provider_duplex" => Some(1),
        "io_read_exact"
        | "io_write_all"
        | "io_provider_filesystem"
        | "io_provider_mutable_filesystem"
        | "io_provider_slot"
        | "io_provider_filesystem_slot"
        | "io_provider_mutable_filesystem_slot"
        | "io_provider_reader_slot"
        | "io_provider_writer_slot"
        | "io_provider_seeker_slot"
        | "io_provider_reader_at_slot"
        | "io_provider_writer_at_slot"
        | "io_provider_closer_slot"
        | "io_provider_syncer_slot"
        | "io_provider_truncater_slot" => Some(2),
        "io_copy" | "io_provider_resource" => Some(3),
        _ => None,
    }
}
