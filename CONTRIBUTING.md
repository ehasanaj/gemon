# Contributing to Gemon

Thank you for considering contributing to Gemon! I welcome contributions from the community to help improve and expand this project. Please follow these guidelines to ensure a smooth contribution process.

## How to Contribute

### Reporting Issues

If you find any bugs or have feature requests, please open an issue on GitHub. When reporting an issue, please include:

* A clear and descriptive title.
* A detailed description of the issue or feature request.
* Steps to reproduce the issue (if applicable).
* Any relevant logs, screenshots, or code snippets.

### Forking the Repository

1. Fork the repository by clicking the "Fork" button on the top right of the project page.

2. Clone your forked repository to your local machine:

```sh
git clone https://github.com/your-username/gemon.git
cd gemon
```

3. Add the original repository as an upstream remote:

```sh
git remote add upstream https://github.com/ehasanaj/gemon.git
git fetch upstream
```

### Creating a Branch

Before starting your work, create a new branch for your feature or bugfix:

```sh
git checkout -b feature/your-feature-name
```

Use a descriptive name for your branch to reflect the changes you are making.

### Making Changes

* Ensure your code follows the existing code style and conventions.
* Write clear and concise commit messages.
* Test your changes thoroughly.

### Submitting a Pull Request

1. Push your changes to your forked repository:

```sh
git push origin feature/your-feature-name
```

2. Open a pull request to the main branch of the original repository. In your pull request, please include:

* A clear and descriptive title.
* A detailed description of the changes you made and why.
* Any relevant issues or pull requests that your changes address.

3. Ensure your branch is up-to-date with the latest changes from the main branch:

```sh
git fetch upstream
git rebase upstream/main
```

4. If your pull request is related to an open issue, link the issue in your pull request description.

### Reviewing and Merging

* Your pull request will be reviewed by the project maintainers. Please be responsive to feedback and make any necessary changes.
* Once your pull request is approved, it will be merged into the main branch.
