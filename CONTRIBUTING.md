# Contributing to yats

We welcome contributions from everyone! To ensure a smooth and effective collaboration, please take a moment to review these guidelines.
## Before You Contribute

### 1. Check the Issues Page First:

Before starting any work or opening a Pull Request, please visit our Issues page.

- <b>Find an existing issue:</b> If you find an issue that you'd like to work on, please comment on it to let others know you're taking it. This helps prevent duplicate efforts.

- <b>No relevant issue? Create one!</b> If you have an idea for a new feature, a bug report, or a suggestion for improvement that isn't already listed, please open a new issue first. Describe your idea or bug clearly. This allows for discussion and ensures that your contribution aligns with the project's goals.

### 2. Wait for Discussion (if applicable): 
For new features or significant changes, it's often helpful to have a brief discussion on the issue before diving into coding.
This can save time and effort in the long run.

## Standard Contribution Workflow

Once you've identified an issue (or created a new one and discussed it), you can follow the standard GitHub workflow:

### 1. Fork the Repository:
Click the "Fork" button at the top right of the repository page to create a copy of this repository in your GitHub account.

### 2. Clone Your Fork:
Clone your forked repository to your local machine:

```bash
git clone https://github.com/[YourGitHubUsername]/[YourRepositoryName].git
```

### 3. Create a New Branch:
Navigate into your cloned repository and create a new branch for your changes. Please use a descriptive branch name (e.g., `feature/add-dark-mode`, `bugfix/fix-login-error`, `docs/update-readme`).

```bash
cd [YourRepositoryName]
git checkout -b your-branch-name
```

### 4. Make Your Changes:
Implement your changes, fix the bug, or add the new feature.

### 5. Commit Your Changes:
Commit your changes with a clear and concise commit message. If your contribution addresses an issue, please reference it in your commit message (e.g., git commit -m "Fix: Resolved issue #123 - Login button not working").

```bash
git add .
git commit -m "Your descriptive commit message"
```

### 6. Push to Your Fork:
Push your new branch and commits to your forked repository on GitHub:

```bash
git push origin your-branch-name
```

### 7. Open a Pull Request (PR):
Go to your forked repository on GitHub. You should see a "Compare & pull request" button next to your new branch. Click it to open a new Pull Request.

- <b>Provide a clear description:</b> In the PR description, explain the purpose of your changes, what problem they solve, and any relevant details. Reference the issue you are addressing (e.g., "Closes #123").

- <b>Link to the issue:</b> Ensure your PR is linked to the relevant issue on the issues page.

## Code Style and Quality

Please try to adhere to the existing code style of the project.

Write clear, maintainable, and well-commented code.

If applicable, write tests for your changes.

Thank you for contributing to yats! Your efforts are greatly appreciated.