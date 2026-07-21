# Transient Assembly

## Projektbeskrivning

Detta är ett lärandeprojekt som en kompis och jag valde att skapa för att lära oss mer om kompileringsprocessen. Projektet tjänar inget syfte alls, och transient assembly är därmed ett NPL (no-purpose programming language)!

Med detta sagt så innehåller detta källförråd två underprojekt. `tasmc` är kompilatorn som kompilerar transient assembly-kod (.tasm) till transient-bytekod (.tbc), och `tasmvm` är den virtuella maskinen som exekverar transient-bytekoden.

## Installation

> [!NOTE]\
> Denna guiden förutsätter att du har en Rust-kompilator installerad. Rust-kompilatorn krävs endast för att bygga tasmc och tasmvm; transient assembly har inga körtidsberoenden av Rust.

Kompilatorn (tasmc) och den virtuella maskinen (tasmvm) kan båda enkelt installeras med Cargo.
```
$ git clone https://github.com/nordtektiger/transient-asm
$ cargo install --path .
```

## Borttagning

Om du önskar att avinstallera dessa verktyg kan du enkelt göra det med Cargo.
```
$ cargo uninstall --package transient-asm
```

## Exempel

För att köra transient assembly-kod så behöver du först anropa kompilatorn (tasmc) och sedan den virtuella maskinen (tasmvm).
```
$ tasmc ./examples/fibonacci.tasm ./fibonacci-ut.tbc
$ tasmvm ./fibonacci-ut.tbc
```

Språket är i många aspekter likt "vanlig" assembly, med några skillnader i t.ex. hur data defineras och körs. Den takt med vilken ändringar görs i språket innebär att en guide över syntax och bytekod-format omöjligen kan hållas uppdaterad i nuläget. För att lära sig mer om språket så rekommenderas att läsa exempel-programmen i ./examples/, alternativt koden som utgör standardbiblioteket som ligger i ./stlib/.

