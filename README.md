# rain.droplets

Raindex trade-level data collection pipeline.

## Prerequisites

Install Nix

``` sh
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

For automatic shell configuration, install Direnv

``` sh
nix -v flake install nixpkgs#direnv
```

Hook Direnv to your shell, e.g. 

``` sh
# For bash
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
source ~/.bashrc

# For zsh
echo 'eval "$(direnv hook zsh)"' >> ~/.zshrc
source ~/.zshrc
```

Enable direnv for the local copy of the repo

``` sh
direnv allow
```

Alternatively, you can enter the dev environment manually using

``` sh
nix develop
```
