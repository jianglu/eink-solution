# Eink Service Runner

服务启动器

Shawl will inspect the state of your program in order to report the correct status to Windows:

By default, when your program exits, Shawl will restart it if the exit code is nonzero. You can customize this behavior with --(no-)restart for all exit codes and --restart-if(-not) for specific exit codes.

When the service is requested to stop, Shawl sends your program a ctrl-C event, then waits up to 3000 milliseconds (based on --stop-timeout) before forcibly killing the process if necessary.
In either case, if Shawl is not restarting your program, then it reports the exit code to Windows as a service-specific error, unless the exit code is 0 or a code you've configured with --pass.

