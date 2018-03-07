error_chain!{
    foreign_links {
        Git(::git2::Error);
        Serde(::serde_json::Error);
        GPGME(::gpgme::Error);
        IO(::std::io::Error);
    }
}
