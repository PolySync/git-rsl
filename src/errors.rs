error_chain!{
    foreign_links {
        Git(::git2::Error);
        Serde(::serde_json::Error);
        GPGME(::gpgme::Error);
        IO(::std::io::Error);
    }

    errors {
        InvalidRSL {
            description("invalid remote RSL"),
            display("invalid remote RSL"),
        }
        MismatchedNonce {
            description("Nonce not found")
            display("Your nonce is not in the nonce bag, but it is not listed in the last push entry either. Someone may have messed with the RSL in a way that is sketchy.")
        }
        ReadError(string: String) {
            description("Could not open file for reading.")
            display("Read Error: could not open '{}' file for reading.
                    This may be a permissions issue.", string)
        }
    }
}
