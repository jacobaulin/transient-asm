#!/bin/sh

cd $(dirname $0)
komponenter=("tasmc" "tasmvm")

for komponent in ${komponenter[@]}; do
  echo "<=> install.sh: installerar $komponent..."
  cargo install --path ./$komponent > /dev/null 2>&1

  if [ $? -ne 0 ]; then
    echo "<=> install.sh: [fel] kunde ej installera $komponent. försök att installera komponenten manuellt istället."
    exit -1
  fi
done

echo "<=> install.sh: färdig."
