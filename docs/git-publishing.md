# Git Publishing in mITyFactory

## Overview

mITyFactory now supports native Git operations for publishing generated applications to remote repositories **without requiring the GitHub CLI (`gh`)**. All Git operations use the native `git` command which is available on most systems.

## Features

✅ **Works Standalone** - Only requires `git` (no `gh` CLI needed)  
✅ **Works with Any Remote** - GitHub, GitLab, Bitbucket, Azure DevOps, self-hosted  
✅ **Full Git Workflow** - Init, status, stage, commit, push, pull  
✅ **User-Friendly UI** - Intuitive modal interface with tabs  
✅ **Error Handling** - Clear error messages and guidance  

## Prerequisites

- **Git** must be installed on your system
  - Windows: Download from [git-scm.com](https://git-scm.com/download/win)
  - macOS: `brew install git` or download installer
  - Linux: `sudo apt install git` (Debian/Ubuntu) or `sudo yum install git` (RHEL/CentOS)

## Usage

### From the UI

1. **Create or Open a Project**
   - Generate an app using mITyFactory
   - The app will be created in `workspaces/<app-name>/`

2. **Open Git Operations**
   - Click the **"Publish to Git"** button in the Deploy section
   - The Git modal will open

3. **Initialize Repository** (if needed)
   - If the project isn't a Git repo yet, click **"Initialize Git Repository"**

4. **Commit Changes**
   - Go to **"Commit"** tab
   - Click **"Stage All Changes"** to add files
   - Enter a commit message
   - Click **"Commit Changes"**

5. **Configure Remote** (first time)
   - Go to **"Settings"** tab
   - Create a repository on GitHub/GitLab/etc.
   - Copy the repository URL (HTTPS or SSH)
   - Enter remote name (usually `origin`) and URL
   - Click **"Add/Update Remote"**

6. **Push to Remote**
   - Go to **"Publish"** tab
   - Select remote and branch
   - Click **"Push to Remote"**

### From the CLI

For generated projects, you can also use standard Git commands:

```bash
cd workspaces/my-app

# Initialize (if needed)
git init

# Stage and commit
git add .
git commit -m "Initial commit"

# Add remote
git remote add origin https://github.com/user/repo.git

# Push
git push -u origin main
```

## Authentication

### HTTPS with Personal Access Token

For HTTPS URLs, you'll need a personal access token:

**GitHub:**
1. Go to Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Generate new token with `repo` scope
3. Use token as password when pushing

**GitLab:**
1. Go to Preferences → Access Tokens
2. Create token with `write_repository` scope
3. Use token as password when pushing

### SSH Keys

For SSH URLs (`git@github.com:user/repo.git`):

1. Generate SSH key: `ssh-keygen -t ed25519 -C "your_email@example.com"`
2. Add key to your Git provider (GitHub: Settings → SSH and GPG keys)
3. Use SSH URL when adding remote

## Architecture

### Backend (Rust)

- **`mity_core::git`** - Git operations module
  - `GitOps` - Main Git operations struct
  - Uses native `git` command via `std::process::Command`
  - Platform-independent (Windows/macOS/Linux)

### Tauri Commands

Exposed commands in `mity_ui/src/commands.rs`:

- `git_is_available()` - Check if Git is installed
- `git_get_repo_info(project_path)` - Get repository information
- `git_init(project_path)` - Initialize repository
- `git_get_status(project_path)` - Get working tree status
- `git_add_all(project_path)` - Stage all changes
- `git_commit(project_path, message)` - Create commit
- `git_list_remotes(project_path)` - List configured remotes
- `git_add_remote(project_path, name, url)` - Add/update remote
- `git_remove_remote(project_path, name)` - Remove remote
- `git_push(project_path, remote, branch, force)` - Push to remote
- `git_pull(project_path, remote, branch)` - Pull from remote

### Frontend (Alpine.js)

- **`git` state object** - Manages Git UI state
- **Git modal** - 4 tabs: Status, Commit, Publish, Settings
- **Methods:**
  - `openGitModal()` - Open modal and load repo info
  - `gitInit()` - Initialize repository
  - `gitAddAll()` - Stage all files
  - `gitCommit()` - Commit with message
  - `gitAddRemote()` - Configure remote
  - `gitPush()` - Push to remote
  - `gitPull()` - Pull from remote

## Troubleshooting

### "Git is not installed"

**Solution:** Install Git from [git-scm.com](https://git-scm.com/) and restart mITyFactory.

### "Authentication failed" when pushing

**Solutions:**
- **HTTPS:** Use personal access token instead of password
- **SSH:** Ensure SSH key is added to your Git provider
- Check that the remote URL is correct

### "failed to push some refs"

This means the remote has commits you don't have locally.

**Solution:**
```bash
# Pull first (may need to resolve conflicts)
git pull origin main --rebase

# Then push
git push origin main
```

### "Repository not initialized"

**Solution:** Click "Initialize Git Repository" in the Git modal, or run `git init` in the project directory.

## Comparison with GitHub CLI Approach

| Feature | Native Git (Current) | GitHub CLI (`gh`) |
|---------|---------------------|-------------------|
| Installation | Git only (usually pre-installed) | Requires `gh` CLI installation |
| Remotes | Any Git server | GitHub only |
| Authentication | Git credentials (tokens/SSH) | GitHub OAuth |
| Repository Creation | Manual (via web UI) | Automated via CLI |
| Dependencies | Minimal | Additional tool |
| Reliability | ✅ Stable, standard Git | Depends on `gh` availability |

## Future Enhancements

Potential improvements:

- [ ] **GitHub API Integration** - Create repositories directly from UI
- [ ] **GitLab/Bitbucket APIs** - Support for other providers
- [ ] **Branch Management** - Create, switch, merge branches
- [ ] **Credential Storage** - Securely store Git credentials
- [ ] **Diff Viewer** - View file changes before committing
- [ ] **Commit History** - Browse past commits

## Related Files

- [`crates/mity_core/src/git.rs`](../../crates/mity_core/src/git.rs) - Git operations implementation
- [`crates/mity_ui/src/commands.rs`](../../crates/mity_ui/src/commands.rs) - Tauri command handlers
- [`crates/mity_ui/dist/app.js`](../../crates/mity_ui/dist/app.js) - Frontend logic
- [`crates/mity_ui/dist/index.html`](../../crates/mity_ui/dist/index.html) - UI modal

## Support

For issues or questions:
- Check [GitHub Issues](https://github.com/mityfactory/mityfactory/issues)
- Review the [Quick Start Guide](../quickstart.md)
- Consult Git documentation: [git-scm.com/doc](https://git-scm.com/doc)
