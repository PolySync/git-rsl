error_chain!{
    foreign_links {
        Git(::git2::Error);
        Serde(::serde_json::Error);
        IO(::std::io::Error);
    }
}
