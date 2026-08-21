# DevDock Project Commands

DevDock supports package scripts and explicit commands stored in `.lumina/project.json`.

## Python Project

Example for a Windows Python project with a virtual environment:

```json
{
  "schemaVersion": 2,
  "name": "ami-insight",
  "types": ["python"],
  "runtimes": {
    "python": {
      "interpreter": ".venv\\Scripts\\python.exe"
    }
  },
  "commands": [
    {
      "id": "api",
      "name": "API service",
      "executor": "python-module",
      "module": "uvicorn",
      "args": ["app:app", "--host", "127.0.0.1", "--port", "8000"],
      "workingDirectory": ".",
      "environment": {
        "PYTHONUNBUFFERED": "1"
      },
      "runPolicy": "singleton"
    },
    {
      "id": "start-cmd",
      "name": "CMD startup",
      "executor": "cmd",
      "script": "scripts\\start.cmd",
      "args": [],
      "runPolicy": "singleton"
    },
    {
      "id": "build",
      "name": "Build",
      "executor": "powershell",
      "script": "scripts\\build.ps1",
      "args": [],
      "runPolicy": "singleton"
    }
  ],
  "defaults": {
    "commandId": "api"
  }
}
```

Commands use their actual process state instead of a configured service/task category. A command that remains running can be stopped or restarted; a command that exits can be run again while retaining its latest exit status and logs. Package scripts do not need this file and are exposed as `package:<script>` commands.

Schema v1 files remain readable. The next save through DevDock migrates `defaults.serviceCommandId` to `defaults.commandId` and removes legacy `kind` fields without changing `package.json`.

Automatic Python discoveries are candidates only. They must be added through the DevDock configuration drawer before they can run.

Scripts must stay in the project directory and should remain in the foreground. Avoid using `start` inside a CMD file because it detaches the child process from DevDock's managed process tree.
