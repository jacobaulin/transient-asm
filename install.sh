#!/bin/sh

cd $(dirname $0)
komponenter=("tasmc" "tasmvm")

# installerar alla komponenter.
for komponent in ${komponenter[@]}; do
  echo "<=> install.sh: installerar $komponent..."
  cargo install --path ./$komponent > /dev/null 2>&1

  if [ $? -ne 0 ]; then
    echo "<=> install.sh: [fel] kunde ej installera $komponent. försök att installera komponenten manuellt istället."
    exit -1
  fi
done

# hjälpsam funktion som startar ett nix-skal med rätt filsökväg.
if uname -a | grep -q "NixOS" && command -v nix-shell > /dev/null 2>&1; then
  if ! echo "$PATH" | grep -q "$HOME/.cargo/bin/"; then
    echo "<=> install.sh: justerar filsökvägen... <nixos>"
    nix-shell --command $SHELL -E "let
      nixpkgs = import <nixpkgs> {};
    in
      nixpkgs.mkShell {
        shellHook = ''
          export PATH=$HOME/.cargo/bin/:$PATH
        '';
      }"
  else
    echo "<=> install.sh: filsökvägen redan justerad <nixos>"
  fi
fi

echo "<=> install.sh: färdig."
