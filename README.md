# agychat (Antigravity Chat Manager)

Herramienta de escritorio y línea de comandos para **ramificar, duplicar, exportar e importar** conversaciones en **Google Antigravity**, manteniendo el historial completo de análisis, transcripciones y artefactos sin necesidad de volver a leer ni indexar los contenidos del proyecto.

Cuenta con **interfaz de consola (CLI)** y una **aplicación de escritorio gráfica (GUI en PyQt6)** con soporte de modo oscuro, arrastrar y soltar (*drag & drop*) e integración en el gestor de tareas del sistema.

---

## 📌 Motivación

Al trabajar con asistentes agentic en proyectos medianos o grandes, el inicio de una conversación suele implicar un proceso intensivo de lectura e indexación del espacio de trabajo (código fuente, documentación, dependencias).

Cuando se desea:
* Probar una solución alternativa o refactor sin perder el contexto ganado en una rama de chat.
* **Compartir un chat completo con un colega** para que continúe trabajando desde el mismo punto.
* **Mover tu contexto de una máquina a otra** (ej. de tu PC de escritorio a una laptop).

Comenzar un chat nuevo obliga al modelo a escanear nuevamente todo el repositorio. Esta herramienta cubre ese vacío permitiendo clonar hilos localmente o empaquetarlos en archivos portátiles (`.agychat`) de forma visual o por terminal.

---

## ✨ Características

* **Interfaz Gráfica de Escritorio (GUI):** Ventana nativa en PyQt6 con tema oscuro, buscador instantáneo, explorador de archivos nativo y soporte para arrastrar y soltar (*drag & drop*) archivos `.agychat`.
* **Integración en el Sistema:** Lanzador `.desktop` con ícono vectorial que permite abrir la app desde el menú de inicio y verla en el gestor de tareas, barra de tareas y selector `Alt + Tab`.
* **Ramificación en vivo:** Se conecta directamente con el motor en memoria de Antigravity (`language_server`). La conversación duplicada o importada aparece de inmediato en la barra lateral sin reiniciar la aplicación ni recargar la ventana.
* **Exportación portátil (.agychat):** Empaqueta la base de trayectoria SQLite, metadatos, transcripciones y artefactos del `brain` en un único archivo comprimido ultra liviano.
* **Importación inteligente:**
  * Adapta de forma automática las rutas de inicio (`$HOME`) y repositorios entre diferentes usuarios o computadoras.
  * Permite asociar explícitamente el chat al workspace local en el equipo de destino (`-w` / `--workspace`).
  * Asocia el chat al proyecto correspondiente en la máquina receptora.
  * Prevención automática de colisiones de identificador (`UUID`).
* **Búsqueda flexible:** Permite ubicar conversaciones por título exacto, coincidencia parcial o ID único.

---

## 🚀 Instalación

1. Clonar el repositorio y acceder a la carpeta:

```bash
git clone https://github.com/Tiago-Pujia/agychat.git
cd agychat
```

2. Otorgar permisos de ejecución y ejecutar el instalador:

```bash
chmod +x install.sh
./install.sh
```

El script se encarga automáticamente de:
* Registrar el comando **`agychat`** en `~/.local/bin` (junto con alias de compatibilidad).
* Instalar el icono vectorial en `~/.local/share/icons/hicolor/scalable/apps/agychat.svg`.
* Registrar el lanzador de escritorio en `~/.local/share/applications/agychat.desktop`.

---

## 🖥️ Interfaz Gráfica (GUI)

Puedes abrir la aplicación gráfica de dos formas:
1. **Desde la terminal:**
   ```bash
   agychat -g
   # o con la opción larga:
   agychat --gui
   ```
2. **Desde el menú de aplicaciones:** Presiona la tecla `Super` (Windows) y escribe **"agychat"**.

### Funciones de la ventana gráfica:
* **Tabla interactiva:** Muestra todas las conversaciones, carpetas de proyecto, cantidad de pasos y fechas.
* **Filtrado en vivo:** Escribe en el buscador superior para filtrar chats instantáneamente.
* **Duplicar con doble clic:** Haz doble clic sobre cualquier chat o usa el botón **Duplicar (-d)** para ramificarlo al instante con un nuevo nombre.
* **Exportar con diálogo nativo:** Elige la carpeta donde guardar el `.agychat` mediante el explorador de archivos del sistema.
* **Importar y Drag & Drop:** Haz clic en **Importar** o simplemente arrastra un archivo `.agychat` dentro de la ventana para cargarlo en caliente.
* **Eliminar con confirmación:** Selecciona un chat y presiona **Eliminar (-D)** (o usa el clic derecho) para retirarlo de Antigravity y liberar espacio en disco de forma segura.

---

## 💻 Ejemplos de Uso en Consola (CLI)

### 1. Consultar ayuda
```bash
agychat -h
agychat --help
```

---

### 2. Listar conversaciones existentes
Para consultar los títulos y UUIDs de todos los chats registrados actualmente en Antigravity:

```bash
agychat -l
agychat --list
```

---

### 3. Duplicar una conversación (en caliente)

* **Copia automática con sufijo `(Copia)`:**
  ```bash
  agychat -d "Analisis Inicial"
  agychat --duplicate "Analisis Inicial"
  ```

* **Copia asignando un nuevo título:**
  ```bash
  agychat -d "Analisis Inicial" -t "Modulo 2 - Refactor"
  agychat --duplicate "Analisis Inicial" --title "Modulo 2 - Refactor"
  ```

> **Nota:** La búsqueda admite coincidencias parciales de título o el UUID de la conversación.

---

### 4. Exportar una conversación a un archivo portátil

Para empaquetar un chat en un archivo portátil (`.agychat`):

```bash
agychat -e "Analisis Inicial"
agychat --export "Analisis Inicial"
```
*(Generará automáticamente `Analisis_Inicial.agychat`).*

También puedes especificar el nombre o ruta del archivo de salida con `-o` o `--output`:

```bash
agychat -e "Analisis Inicial" -o ~/Descargas/mi_chat.agychat
agychat --export "Analisis Inicial" --output ~/Descargas/mi_chat.agychat
```

---

### 5. Importar una conversación en otra computadora

Una vez transferido el archivo a la otra máquina:

```bash
# Importación estándar (detecta automáticamente el workspace si el repo coincide):
agychat -i mi_chat.agychat
agychat --import mi_chat.agychat

# Importación asignando un título nuevo:
agychat -i mi_chat.agychat -t "Investigación Traída de Laptop"

# Importación vinculando a la ruta del proyecto local:
agychat -i mi_chat.agychat -w ~/Proyectos/mi-repositorio

# Forzar la generación de un nuevo UUID si hubiera conflicto:
agychat -i mi_chat.agychat --new-id
```

### 6. Eliminar una conversación

Elimina permanentemente una conversación tanto en Antigravity (descargándola de memoria) como de la base de datos y disco:

```bash
# Solicita confirmación antes de eliminar:
agychat -D "Analisis Inicial"
agychat --delete "Analisis Inicial"

# Eliminar directamente sin confirmación:
agychat -D "Analisis Inicial" -f
agychat --delete "Analisis Inicial" --force
```

---

## 📖 Referencia Completa de Banderas (Flags)

| Bandera Corta | Bandera Larga | Argumento | Descripción |
|---|---|---|---|
| `-g` | `--gui` | *(ninguno)* | Abre la aplicación de escritorio gráfica interactiva. |
| `-h` | `--help` | *(ninguno)* | Muestra la ayuda interactiva en consola. |
| `-l` | `--list` | *(ninguno)* | Lista todos los chats disponibles con su UUID. |
| `-d` | `--duplicate` | `<CHAT>` | Duplica un chat en caliente por título o ID. |
| `-e` | `--export` | `<CHAT>` | Exporta un chat a archivo `.agychat`. |
| `-i` | `--import` | `<ARCHIVO>` | Importa un chat desde archivo `.agychat`. |
| `-D` | `--delete` | `<CHAT>` | Elimina permanentemente una conversación. |
| `-f` | `--force` | *(ninguno)* | No solicita confirmación al eliminar un chat. |
| `-t` | `--title` | `<TITULO>` | Título personalizado al duplicar o importar. |
| `-o` | `--output` | `<ARCHIVO>` | Ruta del archivo de salida al exportar. |
| `-w` | `--workspace` | `<RUTA>` | Ruta del proyecto local en destino al importar. |
| | `--new-id` | *(ninguno)* | Fuerza la asignación de un nuevo UUID al importar. |
| | `--overwrite` | *(ninguno)* | Sobrescribe el chat si el UUID ya existe en destino. |

---

## 📋 Requisitos del Sistema

* **Sistema Operativo:** Linux
* **Python:** 3.8 o superior
* **PyQt6:** Para la interfaz gráfica (generalmente preinstalado o `sudo apt install python3-pyqt6` / `pip install PyQt6`).
* **Google Antigravity:** Instalado. (Para duplicación en vivo o carga inmediata al importar, debe encontrarse en ejecución; de lo contrario, la importación se registrará para el próximo inicio).

---

## 🗑️ Desinstalación

Para retirar el comando, el lanzador `.desktop`, el icono y los enlaces simbólicos del sistema:

```bash
chmod +x uninstall.sh
./uninstall.sh
```
