# Архитектура

![Архитектура x86-native](../assets/architecture.png)

Проект разделён на четыре слоя.

| Слой | Назначение |
| --- | --- |
| Приложение и консоль | Нативные Rust-программы, которые настраивают машину и показывают терминальный вывод. |
| Public API | `Image`, `Resource`, `SavedState`, `MachineConfig` и `Machine`. |
| Execution boundary | `ExecutionBackend` для CPU, памяти, прерываний и устройств. |
| Platform adapters | Файловая система, сеть и терминал Linux, macOS и Windows. |

Host-слой владеет ресурсами и lifecycle state, а execution backend отвечает за выполнение инструкций и семантику устройств. Такое разделение сохраняет переносимость crate и исключает browser-specific assumptions.

Редактируемый исходник схемы находится в [`architecture.mmd`](../assets/architecture.mmd). Его можно отрендерить через `manus-render-diagram` или любой Mermaid-compatible инструмент.
