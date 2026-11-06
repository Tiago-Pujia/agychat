# agychat (Antigravity Chat Manager)

Herramienta de línea de comandos para **ramificar, duplicar, exportar e importar** conversaciones en **Google Antigravity**, manteniendo el historial completo de análisis, transcripciones y artefactos sin necesidad de volver a leer ni indexar los contenidos del proyecto.

---

## 📌 Motivación

Al trabajar con asistentes agentic en proyectos medianos o grandes, el inicio de una conversación suele implicar un proceso intensivo de lectura e indexación del espacio de trabajo (código fuente, documentación, dependencias).

Cuando se desea:
* Probar una solución alternativa o refactor sin perder el contexto ganado en una rama de chat.
* **Compartir un chat completo con un colega** para que continúe trabajando desde el mismo punto.
* **Mover tu contexto de una máquina a otra** (ej. de tu PC de escritorio a una laptop).

Comenzar un chat nuevo obliga al modelo a escanear nuevamente todo el repositorio. Esta herramienta cubre ese vacío permitiendo clonar hilos localmente o empaquetarlos en archivos portátiles (`.agychat`) con comandos simples y directos.

---

## ✨ Características

* **Ramificación en vivo:** Se conecta directamente con el motor en memoria de Antigravity (`language_server`). La conversación duplicada o importada aparece de inmediato en la barra lateral sin reiniciar la aplicación ni recargar la ventana.
* **Exportación portátil (.agychat):** Empaqueta la base de trayectoria SQLite, metadatos, transcripciones y artefactos del `brain` en un único archivo comprimido ultra liviano.
* **Importación inteligente:**
  * Adapta de forma automática las rutas de inicio (`$HOME`) y repositorios entre diferentes usuarios o computadoras.
  * Permite asociar explícitamente el chat al workspace local en el equipo de destino (`-w` / `--workspace`).
  * Asocia el chat al proyecto correspondiente en la máquina receptora.
  * Prevención automática de colisiones de identificador (`UUID`).
* **Búsqueda flexible:** Permite ubicar conversaciones por título exacto, coincidencia parcial o ID único.
* **Sin dependencias externas:** Desarrollado utilizando exclusivamente la biblioteca estándar de Python 3.

---

## 🚀 Instalación

1. Clonar el repositorio y acceder a la carpeta:

```bash
git clone https://github.com/Tiago-Pujia/Antigravity-Duplicate-Chat.git
cd Antigravity-Duplicate-Chat
```

2. Otorgar permisos de ejecución y ejecutar el instalador:

```bash
chmod +x install.sh
./install.sh
```

El script registrará el comando principal **`agychat`** en `~/.local/bin` (junto con los alias `antigravity-chat` y `duplicate-chat-antigravity`).

---

## 💻 Ejemplos de Uso

### 1. Consultar ayuda
```bash
agychat -h
# o con la opción larga:
agychat --help
```

---

### 2. Listar conversaciones existentes
Para consultar los títulos y UUIDs de todos los chats registrados actualmente en Antigravity:

```bash
agychat -l
# o con la opción larga:
agychat --list
```

---

### 3. Duplicar una conversación (en caliente)

* **Copia automática con sufijo `(Copia)`:**
  ```bash
  agychat -d "Analisis Inicial"
  # o con la opción larga:
  agychat --duplicate "Analisis Inicial"
  ```

* **Copia asignando un nuevo título:**
  ```bash
  agychat -d "Analisis Inicial" -t "Modulo 2 - Refactor"
  # o con la opción larga:
  agychat --duplicate "Analisis Inicial" --title "Modulo 2 - Refactor"
  ```

> **Nota:** La búsqueda admite coincidencias parciales de título o el UUID de la conversación.

---

### 4. Exportar una conversación a un archivo portátil

Para empaquetar un chat en un archivo portátil (`.agychat`):

```bash
agychat -e "Analisis Inicial"
# o con la opción larga:
agychat --export "Analisis Inicial"
```
*(Generará automáticamente `Analisis_Inicial.agychat`).*

También puedes especificar el nombre o ruta del archivo de salida con `-o` o `--output`:

```bash
agychat -e "Analisis Inicial" -o ~/Descargas/mi_chat.agychat
# o con la opción larga:
agychat --export "Analisis Inicial" --output ~/Descargas/mi_chat.agychat
```

---

### 5. Importar una conversación en otra computadora

Una vez transferido el archivo a la otra máquina:

```bash
# Importación estándar (detecta automáticamente el workspace si el repo coincide):
agychat -i mi_chat.agychat
# o con la opción larga:
agychat --import mi_chat.agychat

# Importación asignando un título nuevo:
agychat -i mi_chat.agychat -t "Investigación Traída de Laptop"

# Importación vinculando a la ruta del proyecto local:
agychat -i mi_chat.agychat -w ~/Proyectos/mi-repositorio

# Forzar la generación de un nuevo UUID si hubiera conflicto:
agychat -i mi_chat.agychat --new-id
```

> **Carga en vivo:** Si Antigravity está abierto en la máquina de destino, el chat aparecerá de inmediato en la barra lateral. Si está cerrado, estará disponible la próxima vez que abras la aplicación.

---

## 📖 Referencia Completa de Banderas (Flags)

| Bandera Corta | Bandera Larga | Argumento | Descripción |
|---|---|---|---|
| `-h` | `--help` | *(ninguno)* | Muestra la ayuda interactiva. |
| `-l` | `--list` | *(ninguno)* | Lista todos los chats disponibles con su UUID. |
| `-d` | `--duplicate` | `<CHAT>` | Duplica un chat en caliente por título o ID. |
| `-e` | `--export` | `<CHAT>` | Exporta un chat a archivo `.agychat`. |
| `-i` | `--import` | `<ARCHIVO>` | Importa un chat desde archivo `.agychat`. |
| `-t` | `--title` | `<TITULO>` | Título personalizado al duplicar o importar. |
| `-o` | `--output` | `<ARCHIVO>` | Ruta del archivo de salida al exportar. |
| `-w` | `--workspace` | `<RUTA>` | Ruta del proyecto local en destino al importar. |
| | `--new-id` | *(ninguno)* | Fuerza la asignación de un nuevo UUID al importar. |
| | `--overwrite` | *(ninguno)* | Sobrescribe el chat si el UUID ya existe en destino. |

---

## 📋 Requisitos del Sistema

* **Sistema Operativo:** Linux
* **Python:** 3.8 o superior
* **Google Antigravity:** Instalado. (Para duplicación en vivo o carga inmediata al importar, debe encontrarse en ejecución; de lo contrario, la importación se registrará para el próximo inicio).

---

## 🗑️ Desinstalación

Para retirar el comando y sus enlaces simbólicos del sistema:

```bash
chmod +x uninstall.sh
./uninstall.sh
```
