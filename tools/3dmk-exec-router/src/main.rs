fn main() {}

#[cfg(test)]
mod tests {
    use super::{Profile, Runner};

    #[test]
    fn toolkit_check_prefers_bash() {
        let profile = Profile::from_name("toolkit-check").expect("known profile");
        assert_eq!(profile.preferred_runner(), Runner::Bash);
    }

    #[test]
    fn windows_toolkit_check_prefers_pwsh() {
        let profile = Profile::from_name("windows-toolkit-check").expect("known profile");
        assert_eq!(profile.preferred_runner(), Runner::PowerShellCore);
    }

    #[test]
    fn unknown_profile_is_rejected() {
        assert!(Profile::from_name("not-a-profile").is_none());
    }
}
