mod communication;

#[cfg(test)]
mod tests {
    use crate::communication::ssh;
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn test_ssh() {
        let out = ssh::send_command(&"echo \"hello\"");
        assert_eq!(out, "hello\n");
    }
}
