# stealthlingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | **Tiếng Việt**

Công cụ học ngoại ngữ chạy trong terminal, giúp bạn học một cách kín đáo.

StealthLingo là một công cụ dòng lệnh (CLI) nhỏ viết bằng Rust để luyện từ vựng trong những phút rảnh rỗi: tra một từ, lưu lại, rồi học các phiên 3–5 phút với thẻ ghi nhớ (flashcard), luyện chính tả hoặc nghe rồi viết chính tả. Dữ liệu từ điển lấy từ [Wiktionary](https://en.wiktionary.org/) tiếng Anh qua API chính thức của Wikimedia, không cần khóa API; lịch ôn tập, việc chấm đáp án và tiến độ học của bạn được lưu trong cơ sở dữ liệu SQLite trên máy, nên có thể ôn các từ đã lưu khi không có mạng.

Phiên bản v0.1 chỉ hỗ trợ học tiếng Anh.

## Cài đặt

Trên [trang Releases](https://github.com/viys/stealthlingo/releases) có sẵn chương trình đã biên dịch cho Windows (x64), macOS (Intel và Apple silicon) và Linux (x64 và ARM64); không cần cài Rust. Trình cài đặt sẽ đặt `stealthlingo` và chương trình hỗ trợ liên kết `stealthlingo-link` vào `~/.cargo/bin`, rồi thêm thư mục đó vào `PATH`.

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

macOS và Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

Hoặc tải tệp nén cho hệ điều hành của bạn từ trang Releases, giải nén và giữ `stealthlingo` cùng `stealthlingo-link` trong cùng một thư mục. Trên Linux, việc phát âm thanh dùng ALSA (`libasound2`), vốn có sẵn trên các bản phân phối dành cho máy tính để bàn.

### Biên dịch từ mã nguồn

Cần bộ công cụ Rust bản ổn định gần đây (1.88 trở lên) và một trình biên dịch C (SQLite đã được tích hợp sẵn).

```bash
cargo install --git https://github.com/viys/stealthlingo
# hoặc, từ bản sao cục bộ của kho mã
cargo install --path .
cargo run -- --help
```

Trên Linux, để biên dịch còn cần gói phát triển ALSA (ví dụ `libasound2-dev`).

## Bắt đầu nhanh

```bash
stealthlingo lookup ephemeral               # phát âm và các nghĩa chính (--all để xem tất cả)
stealthlingo add ephemeral --note "phù du"  # lưu từ, kèm ghi chú cá nhân tùy chọn
stealthlingo                                # mở ứng dụng toàn màn hình: luyện tập, tra từ, xem danh sách, thống kê
```

## Giao diện toàn màn hình

Trong terminal, `stealthlingo`, `study` và `review` sẽ mở giao diện toàn màn hình. Màn hình chính cho biết hôm nay cần ôn gì và một menu (`r` ôn tập, `s` thẻ ghi nhớ, `p` chính tả, `l` nghe, `/` tra từ, `w` danh sách từ, `t` thống kê, `q` thoát); cũng có thể dùng `↑`/`↓` (hoặc `k`/`j`) và `Enter`. Danh sách và các trang dài cuộn bằng cùng các phím đó.

- **Esc ẩn mọi thứ ngay lập tức** và hiện lại shell mà bạn đã dùng để mở chương trình; nhấn Esc lần nữa để quay lại. Đồng hồ của phiên học tạm dừng khi đang ẩn.
- **Thẻ ghi nhớ** không cần Enter: `Space` hiện đáp án, `1`–`4` để tự chấm, `a` phát âm từ, `s` bỏ qua, `q` kết thúc phiên.
- **Giọng đọc**: `a` phát giọng mặc định của bạn (thiết lập `accent`) trước; nhấn lại trên cùng một từ sẽ phát giọng còn lại đã được thu âm, và cứ thế luân phiên. Thanh dưới cho biết giọng nào sẽ phát tiếp theo, thanh trên cho biết giọng nào đang phát.
- **Chính tả và nghe**: gõ từ rồi nhấn `Enter`. `Tab` hiện đáp án, `Ctrl+N` bỏ qua. Khi luyện nghe, nhấn `Enter` trên dòng trống để phát lại bản ghi âm, `Ctrl+R` để phát giọng còn lại. Khi trả lời sai, chương trình chỉ ra những chữ cái bị thừa hoặc thiếu.
- **Danh sách từ**: `/` để lọc, `Enter` mở mục từ đã lưu (dùng được khi ngoại tuyến), `a` phát âm, `n` sửa ghi chú, `d` rồi `y` để xóa từ (lưu lại từ đó sẽ khôi phục tiến độ). `PgUp`/`PgDn` và `g`/`G` giúp di chuyển nhanh trong danh sách dài.
- **Tra từ**: gõ một từ rồi nhấn `Enter`; sau đó `s` để lưu hoặc xóa từ, `Tab` chuyển giữa mục từ rút gọn và đầy đủ, `/` để tra từ mới, `q` để quay lại.
- `Ctrl+C` kết thúc phiên đang học, hủy bộ lọc hoặc ghi chú đang sửa, còn trong các trường hợp khác thì thoát chương trình. Mỗi câu trả lời được lưu ngay khi bạn trả lời.
- Giao diện cần terminal tối thiểu 40×12 ký tự. Khi cửa sổ thấp, các khung sẽ thu nhỏ, phần tóm tắt ở màn hình chính gói gọn trong một dòng và menu có thể cuộn, để mục đang chọn và ô nhập đáp án luôn hiển thị.

`--plain` (hoặc khi đầu vào/đầu ra không phải terminal, ví dụ khi dùng pipe) sẽ dùng chế độ hỏi từng dòng được mô tả bên dưới. `NO_COLOR` tắt màu.

## Lệnh

| Lệnh | Chức năng |
|---|---|
| `stealthlingo` | Mở ứng dụng toàn màn hình. Với `--plain`: ôn các từ đến hạn; nếu không có từ nào đến hạn thì học từ mới; nếu danh sách trống thì hướng dẫn cách bắt đầu |
| `lookup <WORD> [--all]` | Tra Wiktionary (làm mới bộ nhớ đệm; khi ngoại tuyến thì dùng bản đã lưu đệm) |
| `add <WORD> [--note TEXT]` | Lưu một từ. Lưu hai lần cũng không sao; `--note` cập nhật ghi chú |
| `remove <WORD>` | Xóa từ khỏi danh sách (dữ liệu từ điển đã lưu đệm và lịch sử trả lời vẫn được giữ) |
| `words` | Liệt kê các từ đã lưu cùng trạng thái và thời điểm đến hạn |
| `search <QUERY>` | Tìm trong các từ đã lưu và ghi chú |
| `study [--mode memory\|spelling\|listening] [--minutes N] [--count N]` | Phiên học |
| `review [--count N] [--minutes N]` | Ôn bằng thẻ ghi nhớ, chỉ với các từ đến hạn |
| `stats` | Số từ đã lưu theo trạng thái, số từ đến hạn, số câu trả lời hôm nay và từ trước đến nay kèm tỉ lệ đúng, lần ôn tiếp theo |
| `audio <WORD> [--accent uk\|us]` | Phát âm (mặc định theo thiết lập `accent`) |
| `config` / `config set <KEY> <VALUE>` | Xem hoặc thay đổi thiết lập và vị trí dữ liệu |
| `links [status\|install\|uninstall]` | Ctrl+nhấp trong `lookup` để phát âm và thêm hoặc xóa từ (Windows) |
| `export <PATH>` / `import <PATH>` | `.json` là bản sao lưu đầy đủ, `.csv` là danh sách từ |

Nếu không có `--minutes` hoặc `--count`, một phiên kéo dài `default_minutes` (5) phút.

`lookup` hiển thị cách phát âm trên một dòng, giọng Anh (UK) trước rồi đến giọng Mỹ (US) (`Pronunciation: UK /njuː/ · US /nu/`), và cho biết giọng của bản ghi âm mà `audio` sẽ phát (hoặc gợi ý `--accent uk|us` khi có cả hai). Mặc định, sau đó chương trình liệt kê ba nghĩa đầu tiên của tối đa ba từ loại. `lookup --all` (hoặc `-a`) hiển thị mọi nghĩa kèm ví dụ, từ đồng nghĩa, từ trái nghĩa và nguồn. Các nghĩa được ngắt dòng theo độ rộng terminal, các dòng tiếp theo được thụt lề.

### Tra từ có thể nhấp (Windows)

```bash
stealthlingo links install   # chạy một lần sau khi cài đặt
```

Lệnh này đăng ký trình xử lý liên kết `stealthlingo://` cho tài khoản của bạn (không cần quyền quản trị). Sau đó, `lookup` biến mỗi cách phát âm có bản ghi âm thành siêu liên kết trong terminal: Ctrl+nhấp vào đó để phát bản ghi âm ở chế độ nền, không mở trình duyệt hay cửa sổ nào. Dòng cuối cùng trở thành `+ Add to word list`, hoặc `Saved in your word list · Remove` với từ đã lưu; Ctrl+nhấp vào đó để thêm hoặc xóa từ mà không cần gõ lệnh (chạy lại `lookup` để thấy thay đổi). Vì các chương trình khác cũng có thể mở những liên kết này, việc xóa từ bằng liên kết sẽ giữ lại tiến độ học: thêm lại từ đó, bằng liên kết hoặc bằng `add`, sẽ khôi phục tiến độ. Ngược lại, lệnh `remove` khiến từ đó bắt đầu lại từ đầu. Tính năng này hoạt động trong các terminal hiển thị được siêu liên kết (Windows Terminal, VS Code / Cursor); ở các terminal khác, `lookup` in văn bản thường kèm các lệnh `add` và `audio`. `links status` cho biết kết quả phát hiện, `FORCE_HYPERLINK=1` (hoặc `0`) ghi đè kết quả phát hiện, và `links uninstall` gỡ trình xử lý. Lỗi được ghi vào `link-errors.log` trong thư mục dữ liệu.

### Chế độ học

Các phím dưới đây dành cho chế độ hỏi từng dòng (`--plain`); giao diện toàn màn hình dùng các phím đơn như đã mô tả ở trên.

- **memory (ghi nhớ)**: xem từ, nhấn Enter để hiện nghĩa, rồi tự đánh giá: `1` Again (lại), `2` Hard (khó), `3` Good (tốt), `4` Easy (dễ). Nhấn `a` để nghe từ.
- **spelling (chính tả)**: đọc định nghĩa (bản thân từ đó được che trong định nghĩa và ví dụ) rồi gõ từ. `?` để bỏ cuộc và xem đáp án.
- **listening (nghe)**: nghe phát âm rồi gõ từ. `r` để phát lại. Chỉ dùng các từ có âm thanh phát âm.

Ở mọi chế độ, `s` bỏ qua từ hiện tại mà không ghi nhận câu trả lời, `q` kết thúc phiên. Ctrl+C cũng kết thúc phiên; nhấn Ctrl+C lần thứ hai sẽ thoát ngay. Mỗi câu trả lời đã gửi được lưu ngay lập tức, nên thoát giữa chừng không làm mất phần đã làm, và câu hỏi chưa trả lời không bao giờ bị tính là sai.

Việc chấm đáp án bỏ qua chữ hoa/chữ thường và khoảng trắng ở đầu và cuối, nhưng dấu nháy đơn và dấu gạch nối vẫn được tính (`well-being` khác `wellbeing`).

### Lịch ôn tập

Một thuật toán SM-2 giản lược quyết định khi nào một từ quay lại:

- **Again**: quay lại sau 10 phút; tiến độ bị đặt lại.
- **Hard**: khoảng cách ngắn hơn bình thường.
- **Good**: 1 ngày, rồi 3 ngày, sau đó khoảng cách tăng theo hệ số dễ (ease factor) của từ.
- **Easy**: khoảng cách dài hơn, và từ đó trở đi từ này được coi là dễ hơn.

Trong luyện chính tả và nghe, trả lời đúng được tính là Good, sai được tính là Again. Từ trả lời sai sẽ xuất hiện thêm một lần nữa ở cuối chính phiên đó. Các từ đến hạn ôn luôn được ưu tiên trước từ mới, và mỗi ngày chỉ đưa vào tối đa `daily_new_limit` (mặc định 10) từ chưa từng học. Các từ có khoảng cách từ 21 ngày trở lên được hiển thị là `mastered` (đã thuộc).

## Cấu hình

```bash
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set http_timeout_secs 10
stealthlingo config set accent us        # ưu tiên phát bản ghi âm giọng Mỹ (mặc định: uk)
```

Từ chỉ có bản ghi âm một giọng thì luôn phát bản ghi âm đó.

### Từ điển

StealthLingo dùng API Wiktionary chính thức của Wikimedia: định nghĩa và ví dụ lấy từ REST API, còn IPA (gắn nhãn theo giọng), từ đồng nghĩa, từ trái nghĩa và bản ghi âm trên Wikimedia Commons (OGG) lấy từ mã nguồn của trang. Khi dạng bạn gõ khác với mục từ trong từ điển (`Ephemeral` và `ephemeral`), dạng bạn gõ sẽ được ghi nhớ, nên `add` và `remove` đều chỉ cùng một từ đã lưu.

Các bản phát triển trước khi phát hành v0.1.0 còn có thể dùng Free Dictionary API hoặc Merriam-Webster. Những từ được lưu đệm từ các nguồn đó vẫn đọc được khi ngoại tuyến, và sẽ được thay bằng dữ liệu Wiktionary ở lần tra tiếp theo (tiến độ học và ghi chú được giữ nguyên). Các thiết lập cũ của chúng trong `config.json` sẽ bị bỏ qua.

## Thư mục dữ liệu

`stealthlingo config` in ra đường dẫn chính xác. Mặc định:

| Hệ điều hành | Vị trí |
|---|---|
| Windows | `%APPDATA%\stealthlingo\data` |
| macOS | `~/Library/Application Support/stealthlingo` |
| Linux | `~/.local/share/stealthlingo` |

Đặt `STEALTHLINGO_HOME` để dùng thư mục khác (tiện cho bản di động hoặc khi thử nghiệm). Thư mục này chứa `stealthlingo.db` (SQLite, có migration theo phiên bản), `config.json` và `audio/` (các tệp phát âm đã tải về).

### Sao lưu và danh sách từ

```bash
stealthlingo export backup.json   # mọi thứ: bộ nhớ đệm, tiến độ, lịch sử trả lời
stealthlingo import backup.json   # gộp; nhập lại cùng một tệp không thay đổi gì
stealthlingo export words.csv     # word, note, status, due_at, definition
stealthlingo import examples/words.csv   # cột "word" (nếu không có thì dùng cột đầu tiên); cột "note" là tùy chọn
```

Khi cùng một từ có ở cả hai phía trong lần nhập JSON, tiến độ được ôn gần đây nhất sẽ được giữ. Nhập CSV sẽ tra từng từ chưa có trong bộ nhớ đệm, nên cần kết nối mạng.

## Hoạt động ngoại tuyến

- Học, ôn tập, `words`, `search` và `stats` không bao giờ dùng mạng.
- `lookup` làm mới dữ liệu từ Wiktionary, và hiển thị bản lưu đệm nếu không kết nối được.
- `add` và `audio` dùng bộ nhớ đệm khi có thể và chỉ kết nối mạng với từ mới.
- Âm thanh phát âm được tải về ở lần phát đầu tiên và được dùng lại sau đó.

## Về dữ liệu từ điển

- Mỗi mục từ kèm URL nguồn và giấy phép, có thể xem bằng `lookup --all`. Nội dung Wiktionary được cấp phép CC BY-SA 4.0.
- Mức độ đầy đủ khác nhau tùy từ: phiên âm, âm thanh, ví dụ, từ đồng nghĩa và trái nghĩa đều có thể thiếu. StealthLingo coi mọi trường là tùy chọn; từ không có âm thanh sẽ bị bỏ qua khi luyện nghe.
- Wiktionary không phải dịch vụ dịch thuật. Gợi ý bằng tiếng Việt (hoặc bất kỳ ngôn ngữ nào khác) đến từ `--note` do chính bạn ghi, không bao giờ đến từ từ điển.
- Dịch vụ này miễn phí và có thể chậm hoặc ngừng hoạt động. Hãy tôn trọng điều khoản sử dụng của Wikimedia, và không phân phối lại hàng loạt dữ liệu đã lưu đệm khi chưa tuân thủ giấy phép.

## Phát triển

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Mã nguồn được chia thành `dictionary` (client và bộ phân tích Wiktionary, ánh xạ sang mô hình `Entry`, kèm bộ giải mã cho dữ liệu được lưu đệm bởi các bản trước khi phát hành), `storage` (SQLite), `learning` (lịch ôn, chấm đáp án, phiên học), `audio`, `commands` và `tui` (giao diện toàn màn hình, xây dựng bằng ratatui). Cả hai giao diện đều dùng chung `learning::session::Session`. Lịch ôn tập là một hàm thuần túy, có bộ kiểm thử riêng trong `tests/scheduling_tests.rs`.

### Phát hành

Các bản phát hành được build bằng [dist](https://github.com/axodotdev/cargo-dist) (`dist-workspace.toml`, `.github/workflows/release.yml`). Tăng `version` trong `Cargo.toml`, commit, rồi đẩy một tag tương ứng:

```bash
git tag v0.2.0
git push origin v0.2.0
```

Sau đó GitHub Actions sẽ build cho mọi nền tảng và đăng các tệp nén cùng trình cài đặt thành một GitHub Release. `dist plan` cho xem trước những gì sẽ được build; sau khi thay đổi `dist-workspace.toml`, chạy `dist generate` để cập nhật workflow.

## Giấy phép

MIT, xem [LICENSE](../../LICENSE).
