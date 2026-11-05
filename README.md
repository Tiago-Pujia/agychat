# Antigravity Duplicate Chat

Herramienta de línea de comandos para **ramificar, duplicar, exportar e importar** conversaciones en **Google Antigravity**, manteniendo el historial completo de análisis, transcripciones y artefactos sin necesidad de volver a leer ni indexar los contenidos del proyecto.

---

## 📌 Motivación

Al trabajar con asistentes agentic en proyectos medianos o grandes, el inicio de una conversación suele implicar un proceso intensivo de lectura e indexación del espacio de trabajo (código fuente, documentación, dependencias).

Cuando se desea:
* Probar una solución alternativa o refactor sin perder el contexto ganado en una rama de chat.
* **Compartir un chat completo con un colega** para que continúe trabajando desde el mismo punto.
* **Mover tu contexto de una máquina a otra** (ej. de tu PC de escritorio a una laptop).

Comenzar un chat nuevo obliga al modelo a escanear nuevamente todo el repositorio. Esta herramienta cubre ese vacío permitiendo clonar hilos localmente o empaquetarlos en archivos portátiles (`.agychat`) con un solo comando.

---

## ✨ Características

* **Ramificación en vivo:** Se conecta directamente con el motor en memoria de Antigravity (`language_server`). La conversación duplicada o importada aparece de inmediato en la barra lateral sin reiniciar la aplicación ni recargar la ventana.
* **Exportación portátil (.agychat):** Empaqueta la base de trayectoria SQLite, metadatos, transcripciones y artefactos del `brain` en un único archivo comprimido ultra liviano.
* **Importación inteligente:**
  * Adapta de forma automática las rutas de inicio (`$HOME`) y repositorios entre diferentes usuarios o computadoras.
  * Permite asociar explícitamente el chat al workspace local en el equipo de destino (`--workspace`).
  * Asocia el chat al proyecto correspondiente en la máquina receptora.
  * Prevención automática de colisiones de identificador (`UUID`).
* **Búsqueda flexible:** Permite ubicar conversaciones por título exacto, coincidencia parcial o ID único.
* **Sin dependencias externas:** Desarrollado utilizando exclusivamente la biblioteca estándar de Python 3.

---

## 🚀 Instalación

1. Clonar el repositorio y acceder a la carpeta:

```bash
git clone https://github.com/usuario/antigravity-duplicate-chat.git
cd antigravity-duplicate-chat
```

2. Otorgar permisos de ejecución y ejecutar el instalador:

```bash
chmod +x install.sh
./install.sh
```

---

## 💻 Ejemplos de Uso

### 1. Duplicación local (en caliente)

Crea una copia idéntica del chat agregando el sufijo `(Copia)`:

```bash
duplicate-chat-antigravity "Analisis Inicial"
```

O define un nuevo título personalizado:

```bash
duplicate-chat-antigravity "Analisis Inicial" "Modulo 2 - Refactor Auth"
```

> **Nota:** La búsqueda admite coincidencias parciales de título o el UUID de la conversación.

---

### 2. Exportar una conversación entre computadoras

Para empaquetar un chat en un archivo portátil (`.agychat`):

```bash
duplicate-chat-antigravity -e "Analisis Inicial"
```
*(Generará automáticamente `Analisis_Inicial.agychat`).*

También puedes especificar el nombre o ruta del archivo de salida:

```bash
duplicate-chat-antigravity -e "Analisis Inicial" -o ~/Descargas/chat_compartido.agychat
```

---

### 3. Importar una conversación en otra máquina

Una vez transferido el archivo a la otra computadora:

```bash
# Importación estándar (detecta automáticamente el workspace si el repo coincide):
duplicate-chat-antigravity -i chat_compartido.agychat

# Importación asignando un título nuevo:
duplicate-chat-antigravity -i chat_compartido.agychat -t "Investigación Traída de Desktop"

# Importación forzando la ruta del proyecto en este equipo:
duplicate-chat-antigravity -i chat_compartido.agychat -w ~/Proyectos/mi-repositorio
```

> **Carga en vivo:** Si Antigravity está abierto en la máquina de destino, el chat aparecerá de inmediato en la barra lateral. Si está cerrado, estará disponible la próxima vez que abras la aplicación.

---

### 4. Listar conversaciones existentes

Para consultar los títulos y UUIDs registrados actualmente en el entorno:

```bash
duplicate-chat-antigravity --list
# o de forma abreviada:
duplicate-chat-antigravity -l
```

---

### 5. Consultar ayuda

```bash
duplicate-chat-antigravity --help
```

---

## 📖 Referencia de Opciones CLI

| Opción | Descripción |
|---|---|
| `-e, --export [CHAT]` | Exporta un chat a archivo comprimido (`.agychat`). |
| `-o, --output ARCHIVO` | Ruta o nombre del archivo de salida al exportar. |
| `-i, --import ARCHIVO` | Importa un chat desde un archivo `.agychat` o `.tar.gz`. |
| `-t, --title TITULO` | Título personalizado al importar o duplicar. |
| `-w, --workspace RUTA` | Ruta del proyecto local a vincular en la máquina de destino. |
| `--new-id` | Fuerza la asignación de un nuevo UUID al importar. |
| `--overwrite` | Sobrescribe el chat si el UUID coincide con uno existente. |
| `-l, --list` | Muestra todos los chats registrados en Antigravity. |
| `-h, --help` | Muestra la ayuda interactiva. |

---

## 📋 Requisitos del Sistema

* **Sistema Operativo:** Linux
* **Python:** 3.8 o superior
* **Google Antigravity:** Instalado. (Para duplicación en vivo o carga inmediata al importar, debe encontrarse en ejecución; de lo contrario, la importación se registrará para el próximo inicio).

---

## 🗑️ Desinstalación

Para retirar el comando del sistema:

```bash
chmod +x uninstall.sh
./uninstall.sh
```
