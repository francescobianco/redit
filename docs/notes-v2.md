# V2 — Differenze tra clone-v2 e original-v2

## Stato attuale

- **original-v2** = MS-DOS EDIT V2 (1999, standalone, `dos/EDIT/V2/EDIT.COM` 69.902 bytes) via dosemu
- **clone-v2** = redit con `--v2` (modalità V2 attuale, semplificata)
- Le capture original-v2 esistenti sono **vuote** (0 byte) — i test non funzionano
- Il clone-v2 attuale NON corrisponde affatto all'original V2

## Cosa abbiamo scoperto dell'original V2

### Schermata iniziale (nessun welcome dialog!)

```
   File  Edit  Search  View  Options  Help
┌───────────────────────────────── UNTITLED1 ──────────────────────────────────┐
│                                                                              ↑
│                                                                              █
│                                                                              ...
│                                                                              ↓
 F1=Help                                             │  Line:1    Col:1
```

Caratteristiche:
- **Nessun welcome dialog** — apre direttamente UNTITLED1
- Il nome del file è **UNTITLED1** (non "UNTITLED" come in V1)

### Menu Bar

```
  File  Edit  Search  View  Options  Help
```

- Ha 6 menu: **File, Edit, Search, View, Options, Help**
- **"View"** è un menu aggiuntivo che non esiste in V1 (tra Search e Options)
- I separatori nei menu usano `├───┤` (come V1)

### File Menu

```
 New
 Open...
 Save
 Save As...
 Close
 ─────────────
 Print...
 ─────────────
 Exit
```

- Include **"Close"** che non c'è in V1

### Edit Menu

```
 Cut             Ctrl+X
 Copy            Ctrl+C
 Paste           Ctrl+V
 Clear           Del
```

- Usa Ctrl+X/Ctrl+V (diverso da V1 che usa Shift+Del/Shift+Ins)

### View Menu

```
 Split Window   Ctrl+F6
 Size Window    Ctrl+F8
 Close Window   Ctrl+F4
 ─────────────────────
 1 UNTITLED1      Alt+1
```

### Search Menu

```
 Find...
 Repeat Last Find   F3
 Replace...
```

### Options Menu

```
 Settings...
 Colors...
```

- **"Settings..."** (non esiste in V1)
- **"Colors..."** (in V1 si chiama "Display...")

### Help Menu

```
 Commands...
 About...
```

- Solo 2 voci (V1 ha Getting Started, Keyboard, About)

### Status bar (normale)

```
 F1=Help                                             │  Line:1    Col:1
```

- Formato: `Line:N    Col:N` (non `00001:001` come V1)
- Usa `│` come separatore
- Il separatore è allineato a destra

### Status bar (dialogo)

```
 F1=Help  Enter=Execute  Esc=Cancel  Tab=Next Field
```

### Editor frame

- **Box border completo** (┌─┐, │, └─┘) — ESATTAMENTE come V1!
- **Filename centrato** nel bordo superiore (tipo ` UNTITLED1 `) — come V1!
- **Scrollbar verticale a destra** (↑, █, █, ..., ↓) — come V1 ma con caratteri diversi!
- **Nessuno scrollbar orizzontale** visibile nell'initial screen

### Colori (da ANSI)

- Menu bar: sfondo grigio chiaro, testo nero
- Frame editor: sfondo blu, bordo bianco
- Area di editing: sfondo blu, testo bianco
- Scrollbar: su sfondo grigio (sembra diverso da V1)
- Status bar: sfondo grigio (??), testo nero
- Nome file nel bordo: invertito rispetto al frame

### Scrollbar verticale

La scrollbar occupa l'ultima colonna a destra del frame editor:
- Riga 1: `↑`
- Righe 2-22: `█` (solitamente) o ` ` (thumb position)
- Riga 23: `↓`

Sembra essere FUORI dal box border — il bordo destro `│` è sostituito dalla scrollbar.

### Settings dialog

```
                                  Settings

                     Tab Stops:    [8........]

                     Printer Port: (•) LPT1    ( ) COM1
                                   ( ) LPT2    ( ) COM2
                                   ( ) LPT3

                   ►  OK  ◄▄       Cancel  ▄       Help  ▄
                    ▀▀▀▀▀▀▀▀      ▀▀▀▀▀▀▀▀▀▀      ▀▀▀▀▀▀▀▀

 F1=Help  Enter=Execute  Esc=Cancel  Tab=Next Field
```

### Colors dialog

TODO: Da catturare

## Differenze chiave clone-v2 vs original-v2

| Aspetto | clone-v2 (attuale) | original-v2 (desiderato) |
|---------|-------------------|------------------------|
| Welcome dialog | Mostra welcome | Nessuno — apre UNTITLED1 |
| Menu | Stessi di V1 (File, Edit, Search, Options, Help) | Aggiunge **View** tra Search e Options |
| Options menu | "Display..." | "Settings..." e "Colors..." |
| Frame editor | Solo bordo superiore e inferiore (┌─┐, └─┘), niente filename | Box border completo con **filename centrato** |
| Scrollbar | Nessuna | Scrollbar verticale a destra (↑/█/↓) |
| Status bar | `F1=Help  Untitled  Ln: N  Col: N  OVR` | `F1=Help  │  Line:N    Col:N` |
| Nome file default | "Untitled" | "UNTITLED1" |
| Status bar separatore | Spazi | `│` + allineato a destra |

## Cambiamenti necessari in redit V2

### 1. src/app/mod.rs
- Aggiungere menu **"View"** con voci Split Window, Size Window, Close Window, etc.
- Rinominare "Display..." in **"Settings..."** e **"Colors..."** in Options menu
- Status bar V2: formato `Line:N    Col:N` con separatore `│`

### 2. src/app/v2.rs
- `render_editor_v2`: deve disegnare un box border completo come V1
  - Bordo superiore con filename centrato: `┌──── UNTITLED1 ────┐`
  - Righe con `│` a sinistra e scrollbar a destra
  - Bordo inferiore: `└────────────────────┘`
- Aggiungere scrollbar verticale (↑, █, ↓)

### 3. src/theme.rs
- I colori V2 probabilmente devono essere più simili a V1 con alcune differenze
- Lo scrollbar V2 ha sfondo/colore diverso?

### 4. Welcome dialog
- V2 non ha welcome dialog → aprire direttamente in Mode::Normal

### 5. tests/run.sh
- V2 non ha welcome dialog → `the welcome dialog is dismissed` deve fare cose diverse
- Il nome file è "UNTITLED1" non "Untitled" o "UNTITLED"

### 6. tests/features/*.feature
- Aggiungere condizionali `When "When I'm on V2"` per scenari V2-specifici

### 7. tests/compare.sh
- Aggiungere supporto per confronto V2 (clone-v2 vs original-v2)

## Note sul menu V2

Le scorciatoie da tastiera originali V2:
- **File**: Alt+F → `f`
- **Edit**: Alt+E → `e`
- **Search**: Alt+S → `s`
- **View**: Alt+V → `v` (NUOVO!)
- **Options**: Alt+O → `o`
- **Help**: Alt+H → `h`

Accelerator letters per i menu (come V1, prima lettera di ogni nome):
- F(0), E(0), S(0), V(0), O(0), H(0)

Shortcut keys V2 (Edit menu):
- Cut: Ctrl+X (Shift+Del in V1)
- Copy: Ctrl+C (Ctrl+Ins in V1)
- Paste: Ctrl+V (Shift+Ins in V1)
- Clear: Del (uguale a V1)

## Azioni immediate

1. Catturare Colors dialog originale V2
2. Catturare Settings dialog originale V2  
3. Catturare About dialog originale V2
4. Catturare Help > Commands originale V2
5. Implementare le differenze nel codice
6. Aggiornare test runner
7. Aggiornare feature file
8. Creare compare.sh V2