# tetosays

Сидел искал пакеты, увидел mikusays — очень круто, но мне надо Тето. Решил сделать сам за вечер. Cowsay-клон с Kasane Teto ASCII-артами и облачком речи. Rust, все арты вшиты в бинарь.

## Установка

~~`yay -S tetosays`~~ — пока нет на AUR

~~`cargo install --git https://github.com/dnfngjtfk/tetosays`~~ — пока только из исходников:

```sh
git clone https://github.com/dnfngjtfk/tetosays
cd tetosays
cargo build --release
```

## Использование

```sh
tetosays [OPTIONS] [TEXT]
```

| Флаг | Что делает |
| ---- | ---------- |
| `[TEXT]` | Текст в облачке. Без аргумента — читает stdin (`-` = принудительно stdin) |
| `-s, --style <STYLE>` | Арт по номеру (см. `--list`). Без флага — случайный из пула |
| `-l, --list` | Показать все стили с превью |
| `-w, --width <WIDTH>` | Макс. ширина облачка, 10–200 (по умолчанию 50) |
| `--align <ALIGN>` | Выравнивание: `center`, `left`, `right` (по умолчанию `center`) |
| `--no-clear` | Не чистить экран (по умолчанию чистит на tty) |
| `--disable <N...>` | Убрать стили из рандомного пула (через `-s` всё равно доступны) |
| `--enable <N...>` | Вернуть стили в пул (бьёт дефолтные исключения и `--disable`) |

Примеры:

```sh
tetosays "hi" -s 1
echo "hello" | tetosays --align left
fortune | tetosays
tetosays "x" --disable 6 --enable 3
```

Вывод всегда занимает всю высоту терминала и центрируется вертикально (в пайпах без паддинга).

## Свои арты

Кидай `.txt` файлы в `~/.config/tetosays/arts/` (или `$XDG_CONFIG_HOME/tetosays/arts/`). Каждый файл — новый стиль с номером после встроенных, сортировка по имени файла. Пустые файлы и не-`.txt` игнорируются.

## Дисклеймер

Фанский некоммерческий проект. Kasane Teto © TWINDRILL.

## Лицензия

MIT — см. [LICENSE](LICENSE).
