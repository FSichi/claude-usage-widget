# Claude usage widget

Una cápsula para la barra de tareas de Windows que muestra el consumo del plan de Claude.

- Muestra solo los límites que tu plan informa: semanal, por modelo, 5 horas y uso extra.
- Cada límite se muestra u oculta desde el panel que se abre con un clic.
- Marca el ritmo esperado, para ver si vas por encima o por debajo de lo que corresponde a esta altura de la ventana. La barra se pone ámbar cuando lo pasás.
- Proyecta dónde vas a terminar si mantenés ese ritmo, o cuándo llegarías al límite.
- Guarda un historial con el consumo de cada día de las últimas dos semanas, y compara los últimos 7 días con los 7 anteriores.
- Avisa con una notificación de Windows al cruzar el 70, 80 o 90% que elijas, al llegar al 95%, y cuando se reinicia un límite que ya te había avisado.
- Ocho temas, y español o inglés.
- Se arrastra a lo largo de la barra, guarda la posición y puede vivir en cualquier monitor con barra de tareas.
- Tiene un modo compacto: un punto y un valor por límite, en una sola fila.
- Es un `.exe` nativo de menos de 1 MB.

## Usarlo

Abrí `claude-usage-widget.exe` y listo. En el primer arranque el widget se deja configurado: activa
el inicio con Windows, crea su acceso en el menú Inicio y registra el mod que lo alimenta. Hace falta
tener instalada la app de Claude.

La marca vertical de cada barra es el ritmo esperado: dónde estaría el consumo si se repartiera
parejo en toda la ventana. Si la barra no llegó a la marca, vas por debajo de ese ritmo.

Para comprobar que las notificaciones llegan a esta PC, abrí el `.exe` con `--test-alert`.

El dato empieza a llegar con la próxima sesión de Claude Code que abras en la app.

## De dónde sale el dato

El widget no se conecta a ninguna red ni lee credenciales: dibuja lo que encuentra en
`%USERPROFILE%\.claude-usage-widget\usage.json`.

Ese archivo lo escribe [`mod/`](mod), un mod de la app de Claude que cada minuto le pide los
límites a la propia app. Va dentro del `.exe`, que lo registra solo con el CLI de Claude Code. Por
eso el dato solo se actualiza mientras haya una sesión de Claude Code activa en la app; si no, el
widget muestra el último dato y avisa que no hay datos nuevos.

Todo vive en la carpeta de usuario y no en `AppData` a propósito: la app de Claude de Microsoft
Store corre empaquetada, y Windows redirige lo que escribe en `AppData` a una carpeta privada que
el widget no ve.

La consulta que usa el mod no es una API pública de Anthropic: si cambia, habrá que ajustar el mod.

## Publicar una versión

Las ramas, los commits, los PR y las versiones van con [gitwiz](https://github.com/FSichi/gitwiz).
Este repo no tiene `package.json`, así que gitwiz toma la versión de la última etiqueta: la de
`Cargo.toml` se sube a mano en la rama del release.

```bash
gitwiz release start --minor
gitwiz release finish --yes
```

La etiqueta `v*` que deja gitwiz dispara [el Action](.github/workflows/release.yml): corre los tests
en Windows, compila el `.exe` con el logo y lo publica en Releases. Si se lanza a mano desde la
pestaña Actions, deja el `.exe` como artefacto de la corrida, sin publicar nada.

El logo se genera con [`assets/make-icon.ps1`](assets/make-icon.ps1).

## Compilar

No hace falta Visual Studio, solo Rust:

```bash
rustup target add x86_64-pc-windows-gnullvm
rustup component add rust-mingw --target x86_64-pc-windows-gnullvm
cargo test
cargo build --release
claude plugin test mod
```

La configuración está en `.cargo/config.toml`. Si el repo está en una carpeta sincronizada, como
OneDrive, conviene definir `build.target-dir` en `~/.cargo/config.toml`. Esta compilación local no
lleva el logo, porque hace falta un compilador de recursos: el `.exe` con logo sale del Action.

## Licencia

[MIT](LICENSE).
