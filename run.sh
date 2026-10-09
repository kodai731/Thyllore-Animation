#!/usr/bin/env bash
set -euo pipefail

# Single entry point for every launch flavour. The actual run scripts live in
# scripts/ (and src/ml/worker/); this file only dispatches.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FULL_ENV_FILE="$REPO_ROOT/.config/curve_copilot_full.env"

usage() {
    cat <<EOF
Usage: ./run.sh <command> [args...]

Every launch command runs in a detached tmux session named
thyllore-<command> and returns at once, so this terminal can keep issuing
commands such as live-dump against the running engine. Pass --no-tmux
(or --on-terminal) before or after the command to run it here instead.
help, live-dump and auto always run on this terminal. The session closes
by itself when the command exits; its output is in the usual log files.
    ./run.sh engine                 # then: ./run.sh live-dump --timings 1s
    tmux attach -t thyllore-engine  # see the engine output (Ctrl-b d to leave)
    ./run.sh engine --no-tmux       # stay on this terminal

Commands:
  engine [private|degrade|full] [cargo args...]
      Launch the engine (cargo run) with a curve copilot mode.
      The engine includes the helm command bar (text control; Read Only by default).
      full sources .config/curve_copilot_full.env (gitignored) for
      THYLLORE_FEEDBACK_TEST_ENDPOINT / THYLLORE_INGEST_TOKEN.
        ./run.sh engine                              # private (default)
        ./run.sh engine degrade
        ./run.sh engine full --features auto-rig
  blender [--mode degrade|full|private] [scene.blend] [args...]
      Build + install the debug addon and launch Blender
      (scripts/run_blender_debug.sh). The mode maps to the addon build mode
      (degrade=A, full=B, private=C; default: degrade). Sources
      .config/curve_copilot_full.env for THYLLORE_FEEDBACK_TEST_ENDPOINT /
      THYLLORE_INGEST_TOKEN (dev builds always bake the test endpoint).
        ./run.sh blender --mode full
  blender --flame [--skip-build] [--release] [--overlap] [--software-gl] [scene.blend]
      Build the flame addon ZIP, install it into a pristine Docker Blender on
      the NVIDIA GPU and open blender/test.blend with a campfire flame already
      added (scripts/blender/flame/launch.sh). Fresh-install checks use blender-verify:
        ./run.sh blender --flame
        ./run.sh blender-verify --zip dist/thyllore_flame-0.0.1-linux_x86_64.zip
  blender --water [--skip-build] [--release] [--software-gl] [scene.blend]
      Build the water addon ZIP, install it into a pristine Docker Blender on
      the NVIDIA GPU and open blender/water.blend with a water torus already
      added (scripts/blender/water/launch.sh):
        ./run.sh blender --water
  blender --wind [--skip-build] [--release] [--software-gl] [scene.blend]
      Build the wind addon ZIP, install it into a pristine Docker Blender on
      the NVIDIA GPU and open blender/wind.blend with a wind tornado already
      added (scripts/blender/wind/launch.sh):
        ./run.sh blender --wind
  blender --lightning [--skip-build] [--release] [--software-gl] [scene.blend]
      Build the lightning addon ZIP, install it into a pristine Docker Blender on
      the NVIDIA GPU and open blender/lightning.blend with a lightning bolt already
      added (scripts/blender/lightning/launch.sh):
        ./run.sh blender --lightning
  blend [--scene PATH.blend] [--software-gl] [args...]
      Open a pristine Docker Blender on the NVIDIA GPU with a new empty scene,
      no addon installed (blender/docker/run_gui.sh --no-install).
  blend --background [blender args...]
      Run the same Docker Blender on the NVIDIA GPU without a window, every
      argument passed to Blender (blender/docker/run_background.sh). That script
      is what BlenderPath in .claude/local/paths.md points to:
        ./run.sh blend --background --python scripts/blender_gltf_roundtrip.py -- in.glb out.glb
  blender-verify [--mode degrade|full|private] [--zip PATH] [args...]
      Launch a pristine Blender GUI in Docker with NO addon installed
      (blender/docker/run_gui.sh --no-install). The ZIP is mounted at
      /zips for manual verification via Edit > Preferences > Add-ons >
      Install from Disk. Nothing persists between runs.
        ./run.sh blender-verify --mode private --zip dist/release_download/release-zip-linux_x86_64/thyllore_animation_curve_copilot_private-0.0.1-linux_x86_64.zip
  auto [args...]
      Launch the Claude auto-mode container (scripts/run_auto_mode.sh).
  worker-smoke [worker-url] [args...]
      Smoke-test the deployed feedback worker (src/ml/worker/smoke.sh). Sources the
      full-mode env file; WORKER_URL is derived from
      THYLLORE_FEEDBACK_TEST_ENDPOINT when not given.
  helm-test [args...]
      Runs parity and e2e tests for the helm router.
        ./run.sh helm-test
  unity-verify
      End-to-end avatar check on a synthetic rig (unity/verify/run.sh):
      Blender builds a cube with a humanoid armature and blend shapes, the
      engine (batch run, needs the GPU and a display) edits and keys the blend
      shapes and exports the sidecar and .anim files, Unity (empty project +
      VRChat SDK via vrc-get) installs the unity/com.thyllore.avatar package and BatchCheck
      compares the sampled animation with the engine's. UNITY_EDITOR /
      UNITY_VERIFY_DIR / BLENDER override the editor binary, the work dir
      (default target/unity_verify) and the Blender launcher (default
      blender/docker/run_background.sh).
  unity --model PATH.fbx [--fresh] [--setup-only] [--software-gl]
      Open the Unity Editor GUI (GameCI 2022.3.22f1 image + VRChat SDK) on
      the NVIDIA GPU with a scene that already holds the model, the sidecar
      applied and an Animator with every .anim the engine exported next to
      the model, so the expression presets can be scrubbed in the Animation
      window (unity/docker/run_gui.sh). The project persists in
      target/unity_gui; UNITY_GUI_DIR overrides it:
        ./run.sh unity --model assets/models/purchased/Shinano_ver1.02/FBX/Shinano.fbx
  live-dump [--tracks] [--pose TIME...] [--set-time T] [--timings [1s]] [--clip NAME] [--out FILE]
      Talk to the engine that is already running (started earlier with
      ./run.sh engine); never launches one. Prints its clips, timeline and
      curve editor state, sampled poses, or frame timings as JSON
      (tools/live_dump.py). --timings 1s starts recording at the moment you
      run it, for 1 s, so load the model and start scrubbing first. One
      request per connection on log/live_dump/engine.sock; the engine
      applies it as a UI command on its main thread, nothing is polled.
        ./run.sh engine                        # once; then, at any time:
        ./run.sh live-dump --tracks --clip "New Clip"
        ./run.sh live-dump --timings 1s        # per-frame CPU ms from now for 1 s
  help
      Show this help.
EOF
}

is_foreground_flag() {
    [[ "${1:-}" == "--no-tmux" || "${1:-}" == "--on-terminal" ]]
}

foreground=0
if is_foreground_flag "${1:-}"; then
    foreground=1
    shift
fi
command="${1:-help}"
shift || true
if is_foreground_flag "${1:-}"; then
    foreground=1
    shift
fi

runs_on_terminal() {
    case "$1" in
        help|-h|--help|live-dump|auto) return 0 ;;
        *) return 1 ;;
    esac
}

run_in_tmux() {
    local session="thyllore-$command"
    if tmux has-session -t "$session" 2>/dev/null; then
        echo "tmux session '$session' is already running; attach with: tmux attach -t $session" >&2
        exit 1
    fi

    local quoted
    quoted="$(printf ' %q' "$command" --no-tmux "$@")"
    tmux new-session -d -s "$session" -c "$REPO_ROOT" "bash ./run.sh$quoted"
    echo "started '$command' in tmux session '$session' (this terminal stays free)"
    echo "  attach: tmux attach -t $session    stop: tmux kill-session -t $session"
}

if [[ "$foreground" -eq 0 ]] && ! runs_on_terminal "$command" && [[ -n "${TMUX:-}" || -t 1 ]]; then
    run_in_tmux "$@"
    exit 0
fi

case "$command" in
    engine)
        exec bash "$REPO_ROOT/scripts/run_engine.sh" "$@"
        ;;
    blender)
        if [[ "${1:-}" == "--flame" ]]; then
            shift
            exec bash "$REPO_ROOT/scripts/blender/flame/launch.sh" "$@"
        fi
        if [[ "${1:-}" == "--water" ]]; then
            shift
            exec bash "$REPO_ROOT/scripts/blender/water/launch.sh" "$@"
        fi
        if [[ "${1:-}" == "--wind" ]]; then
            shift
            exec bash "$REPO_ROOT/scripts/blender/wind/launch.sh" "$@"
        fi
        if [[ "${1:-}" == "--lightning" ]]; then
            shift
            exec bash "$REPO_ROOT/scripts/blender/lightning/launch.sh" "$@"
        fi
        exec bash "$REPO_ROOT/scripts/run_blender_debug.sh" "$@"
        ;;
    blend)
        if [[ "${1:-}" == "--background" ]]; then
            exec bash "$REPO_ROOT/blender/docker/run_background.sh" "$@"
        fi
        exec bash "$REPO_ROOT/blender/docker/run_gui.sh" --no-install "$@"
        ;;
    blender-verify)
        exec bash "$REPO_ROOT/blender/docker/run_gui.sh" --no-install "$@"
        ;;
    auto)
        exec bash "$REPO_ROOT/scripts/run_auto_mode.sh" "$@"
        ;;
    worker-smoke)
        if [[ -r "$FULL_ENV_FILE" ]]; then
            set -a
            # shellcheck disable=SC1090
            source "$FULL_ENV_FILE"
            set +a
        fi
        smoke_endpoint="${THYLLORE_FEEDBACK_TEST_ENDPOINT:-${THYLLORE_FEEDBACK_ENDPOINT:-}}"
        if [[ -z "${WORKER_URL:-}" && -n "$smoke_endpoint" ]]; then
            export WORKER_URL="${smoke_endpoint%/v1/feedback}"
        fi
        exec bash "$REPO_ROOT/src/ml/worker/smoke.sh" "$@"
        ;;
    helm-test)
        exec bash "$REPO_ROOT/scripts/run_helm_test.sh" "$@"
        ;;
    unity-verify)
        exec bash "$REPO_ROOT/unity/verify/run.sh" "$@"
        ;;
    unity)
        exec bash "$REPO_ROOT/unity/docker/run_gui.sh" "$@"
        ;;
    live-dump)
        exec python3 "$REPO_ROOT/tools/live_dump.py" "$@"
        ;;
    help|-h|--help)
        usage
        ;;
    *)
        echo "unknown command: $command" >&2
        usage >&2
        exit 2
        ;;
esac
