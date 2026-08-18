# Архітектура

![Архітектура x86-native](../assets/architecture.png)

Проєкт поділено на чотири шари.

| Шар | Призначення |
| --- | --- |
| Застосунок і консоль | Нативні Rust-програми, які налаштовують машину та показують термінальний вивід. |
| Public API | `Image`, `Resource`, `SavedState`, `MachineConfig` і `Machine`. |
| Execution boundary | `ExecutionBackend` для CPU, пам’яті, переривань і пристроїв. |
| Platform adapters | Файлова система, мережа й термінал Linux, macOS та Windows. |

Host-шар володіє ресурсами й lifecycle state, а execution backend відповідає за виконання інструкцій та семантику пристроїв. Таке розділення зберігає переносимість crate та усуває browser-specific assumptions.

Редагований вихідний файл схеми знаходиться в [`architecture.mmd`](../assets/architecture.mmd). Його можна відрендерити через `manus-render-diagram` або будь-який Mermaid-compatible інструмент.
