#!/usr/bin/env python3
import base64
import json
import mimetypes
import os
import re
import sys
import time
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


BASE_URL = "https://aihubmix.com"
API_KEY = os.environ.get("AIHUBMIX_API_KEY", "").strip()
ROOT = Path(__file__).resolve().parents[1]
OUT_ROOT = ROOT / "generated" / "aihubmix_raw"
REQUESTS_DIR = OUT_ROOT / "requests"
RESPONSES_DIR = OUT_ROOT / "responses"
MEDIA_DIR = OUT_ROOT / "media"
MANIFEST_PATH = OUT_ROOT / "manifest.json"


IMAGE_JOBS = [
    {
        "slug": "rainy-market-photo",
        "model_path": "openai/gpt-image-1.5",
        "payload": {
            "input": {
                "prompt": "A photoreal rainy night market in Taipei, warm lantern light reflecting on wet pavement, candid street photography, 35mm lens feel, layered depth, natural faces, no text",
                "size": "1024x1024",
                "n": 1,
                "quality": "high",
                "output_format": "png",
            }
        },
    },
    {
        "slug": "underwater-library-illustration",
        "model_path": "google/imagen-4.0-fast-generate-001",
        "payload": {
            "input": {
                "prompt": "An underwater library carved into coral, shafts of sunlight through the water, floating dust motes, whimsical but believable environment concept art",
                "numberOfImages": 1,
            }
        },
    },
    {
        "slug": "desert-train-poster",
        "model_path": "qianfan/qwen-image",
        "payload": {
            "input": {
                "prompt": "A retro-futurist poster of a magnetic train crossing a desert canyon at golden hour, bold composition, crisp typography-ready negative space, cinematic color separation",
                "n": 1,
                "size": "1024x1024",
            }
        },
    },
    {
        "slug": "tea-house-courtyard",
        "model_path": "doubao/doubao-seedream-4-5",
        "payload": {
            "input": {
                "prompt": "A quiet tea house courtyard after light rain, polished stone, bamboo shadows, steam rising from cups, ultra-detailed realistic photography, tranquil mood",
                "size": "2K",
                "sequential_image_generation": "disabled",
                "response_format": "url",
                "watermark": True,
            }
        },
    },
]

VIDEO_JOBS = [
    {
        "slug": "origami-bird-liftoff",
        "payload": {
            "model": "wan2.6-t2v",
            "prompt": "An origami crane on a wooden table slowly unfolds into a real white bird and flies toward an open window, gentle morning light, steady cinematic camera",
            "seconds": "5",
            "size": "1280x720",
        },
    },
    {
        "slug": "courtyard-lantern-breeze",
        "payload": {
            "model": "wan2.6-i2v",
            "prompt": "The scene comes alive with a soft breeze moving bamboo leaves and hanging lanterns, steam drifting naturally, subtle camera push-in, calm realistic motion",
            "seconds": "5",
            "size": "1280x720",
        },
        "image_source_slug": "tea-house-courtyard",
    },
]

AUDIO_JOBS = [
    {
        "slug": "museum-guide",
        "payload": {
            "model": "tts-1",
            "voice": "alloy",
            "input": "Welcome to the gallery. Please move slowly through the next room and notice how the lighting changes the mood of the space.",
            "response_format": "mp3",
        },
    },
    {
        "slug": "flight-briefing",
        "payload": {
            "model": "gpt-4o-mini-tts",
            "voice": "verse",
            "input": "Gate A12 now begins boarding for the evening flight to Singapore. Business class and families with small children may board at this time.",
            "response_format": "mp3",
            "instructions": "Speak like a calm premium-airline announcement with clear diction, measured pacing, and a polished public-address tone.",
        },
    },
    {
        "slug": "dual-host-dialogue",
        "payload": {
            "model": "gemini-2.5-flash-preview-tts",
            "voice": "Kore",
            "input": "TTS the following conversation between Lin and Maya:\nLin: We have three shots left before sunset.\nMaya: Then let us take the wide shot first and keep the energy light.\nLin: Perfect, I will cue the camera move on your signal.",
            "response_format": "wav",
            "instructions": "Lin should sound grounded and focused. Maya should sound upbeat and collaborative.",
        },
    },
]


def ensure_dirs() -> None:
    for path in (REQUESTS_DIR, RESPONSES_DIR, MEDIA_DIR):
        path.mkdir(parents=True, exist_ok=True)


def slugify(value: str) -> str:
    value = value.lower().replace("/", "-")
    value = re.sub(r"[^a-z0-9._-]+", "-", value)
    return value.strip("-") or "file"


def save_json(path: Path, data: Any) -> None:
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def load_manifest() -> dict[str, Any]:
    if MANIFEST_PATH.exists():
        return json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    return {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "base_url": BASE_URL,
        "notes": "Files are saved exactly as returned by AIHubMix. No transcoding or metadata stripping is performed.",
    }


def json_request(method: str, url: str, payload: dict[str, Any] | None = None, headers: dict[str, str] | None = None) -> dict[str, Any]:
    req_headers = {
        "Authorization": f"Bearer {API_KEY}",
    }
    if payload is not None:
        req_headers["Content-Type"] = "application/json"
    if headers:
        req_headers.update(headers)
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = Request(url, data=data, method=method, headers=req_headers)
    try:
        with urlopen(request, timeout=600) as response:
            body = response.read().decode("utf-8")
            return json.loads(body)
    except HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {url} failed with {exc.code}: {detail}") from exc
    except URLError as exc:
        raise RuntimeError(f"{method} {url} failed: {exc}") from exc


def binary_request(method: str, url: str, payload: dict[str, Any] | None = None, headers: dict[str, str] | None = None) -> tuple[bytes, str]:
    req_headers = {
        "Authorization": f"Bearer {API_KEY}",
    }
    if payload is not None:
        req_headers["Content-Type"] = "application/json"
    if headers:
        req_headers.update(headers)
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = Request(url, data=data, method=method, headers=req_headers)
    try:
        with urlopen(request, timeout=600) as response:
            return response.read(), response.headers.get_content_type()
    except HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {url} failed with {exc.code}: {detail}") from exc
    except URLError as exc:
        raise RuntimeError(f"{method} {url} failed: {exc}") from exc


def fetch_url(url: str, headers: dict[str, str] | None = None) -> tuple[bytes, str]:
    request = Request(url, headers=headers or {})
    with urlopen(request, timeout=600) as response:
        return response.read(), response.headers.get_content_type()


def infer_ext_from_content_type(content_type: str | None) -> str:
    if not content_type:
        return ".bin"
    guessed = mimetypes.guess_extension(content_type.split(";")[0].strip())
    if guessed == ".jpe":
        return ".jpg"
    return guessed or ".bin"


def infer_ext_from_bytes(data: bytes) -> str:
    if data.startswith(b"\x89PNG\r\n\x1a\n"):
        return ".png"
    if data.startswith(b"\xff\xd8\xff"):
        return ".jpg"
    if data.startswith(b"RIFF") and data[8:12] == b"WAVE":
        return ".wav"
    if data.startswith(b"ID3") or data[:2] == b"\xff\xfb":
        return ".mp3"
    if len(data) >= 8 and data[4:8] == b"ftyp":
        return ".mp4"
    return ".bin"


def refs_from_value(value: Any, found: list[dict[str, str]]) -> None:
    if isinstance(value, dict):
        for key in ("url", "image_url", "audio_url"):
            item = value.get(key)
            if isinstance(item, str) and item.startswith("http"):
                found.append({"kind": "url", "value": item})
        if isinstance(value.get("bytesBase64"), str) and value["bytesBase64"]:
            ref = {"kind": "base64", "value": value["bytesBase64"]}
            if isinstance(value.get("mimeType"), str):
                ref["mime_type"] = value["mimeType"]
            found.append(ref)
        for key in ("b64_json", "base64_json", "base64"):
            item = value.get(key)
            if isinstance(item, str) and item:
                found.append({"kind": "base64", "value": item})
        for nested in value.values():
            refs_from_value(nested, found)
        return
    if isinstance(value, list):
        for item in value:
            refs_from_value(item, found)
        return
    if isinstance(value, str):
        if value.startswith("http"):
            found.append({"kind": "url", "value": value})
        elif value.startswith("data:"):
            found.append({"kind": "data_url", "value": value})


def extract_media_refs(response: dict[str, Any]) -> list[dict[str, str]]:
    found: list[dict[str, str]] = []
    refs_from_value(response, found)
    unique: list[dict[str, str]] = []
    seen: set[tuple[str, str]] = set()
    for ref in found:
        marker = (ref["kind"], ref["value"])
        if marker not in seen:
            unique.append(ref)
            seen.add(marker)
    return unique


def write_bytes(path: Path, data: bytes) -> None:
    path.write_bytes(data)


def file_name(prefix: str, model: str, slug: str, index: int, ext: str) -> str:
    return f"{prefix}_{slugify(model)}_{slugify(slug)}_{index:02d}{ext}"


def image_data_url(path: Path) -> str:
    content = path.read_bytes()
    mime = mimetypes.guess_type(path.name)[0] or "application/octet-stream"
    return f"data:{mime};base64,{base64.b64encode(content).decode('ascii')}"


def selected_stage_names() -> set[str]:
    raw = os.environ.get("AIHUBMIX_STAGES", "images,videos,audio")
    return {part.strip() for part in raw.split(",") if part.strip()}


def filter_jobs(env_name: str, jobs: list[dict[str, Any]]) -> list[dict[str, Any]]:
    raw = os.environ.get(env_name, "").strip()
    if not raw:
        return jobs
    allowed = {part.strip() for part in raw.split(",") if part.strip()}
    return [job for job in jobs if job["slug"] in allowed]


def merge_entries(existing: list[dict[str, Any]], updates: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_slug = {item["slug"]: item for item in existing if "slug" in item}
    for item in updates:
        by_slug[item["slug"]] = item
    return [by_slug[key] for key in sorted(by_slug)]


def generate_images(manifest: dict[str, Any]) -> None:
    images_manifest = []
    for index, job in enumerate(filter_jobs("AIHUBMIX_IMAGE_SLUGS", IMAGE_JOBS), start=1):
        model_path = job["model_path"]
        payload = job["payload"]
        request_name = f"image_{slugify(model_path)}_{slugify(job['slug'])}.request.json"
        response_name = f"image_{slugify(model_path)}_{slugify(job['slug'])}.response.json"
        save_json(REQUESTS_DIR / request_name, payload)
        response = json_request("POST", f"{BASE_URL}/v1/models/{model_path}/predictions", payload)
        save_json(RESPONSES_DIR / response_name, response)
        refs = extract_media_refs(response)
        if not refs:
            raise RuntimeError(f"No image output references found for {model_path}")
        outputs = []
        for out_index, ref in enumerate(refs, start=1):
            if ref["kind"] == "url":
                data, content_type = fetch_url(ref["value"])
                ext = infer_ext_from_content_type(content_type)
            elif ref["kind"] == "data_url":
                header, raw = ref["value"].split(",", 1)
                content_type = header.split(";")[0][5:]
                data = base64.b64decode(raw)
                ext = infer_ext_from_content_type(content_type)
            else:
                data = base64.b64decode(ref["value"])
                ext = infer_ext_from_content_type(ref.get("mime_type")) if ref.get("mime_type") else infer_ext_from_bytes(data)
                if ext == ".bin":
                    ext = infer_ext_from_bytes(data)
            media_name = file_name("image", model_path, job["slug"], out_index, ext)
            media_path = MEDIA_DIR / media_name
            write_bytes(media_path, data)
            outputs.append(str(media_path.relative_to(ROOT)))
        images_manifest.append(
            {
                "slug": job["slug"],
                "model_path": model_path,
                "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
                "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
                "outputs": outputs,
            }
        )
        print(f"Generated image via {model_path}: {', '.join(outputs)}", flush=True)
    manifest["images"] = merge_entries(manifest.get("images", []), images_manifest)


def resolve_image_source(manifest: dict[str, Any], slug: str) -> Path:
    for item in manifest.get("images", []):
        if item["slug"] == slug and item["outputs"]:
            return ROOT / item["outputs"][0]
    raise RuntimeError(f"Unable to find image output for slug={slug}")


def poll_video(video_id: str) -> dict[str, Any]:
    status_url = f"{BASE_URL}/v1/videos/{video_id}"
    deadline = time.time() + 60 * 40
    while True:
        status = json_request("GET", status_url)
        current = status.get("status")
        if current == "completed":
            return status
        if current == "failed":
            raise RuntimeError(f"Video job {video_id} failed: {json.dumps(status, ensure_ascii=False)}")
        if time.time() > deadline:
            raise RuntimeError(f"Video job {video_id} timed out")
        print(f"Waiting on video {video_id}: status={current}", flush=True)
        time.sleep(15)


def generate_videos(manifest: dict[str, Any]) -> None:
    videos_manifest = []
    for job in filter_jobs("AIHUBMIX_VIDEO_SLUGS", VIDEO_JOBS):
        payload = dict(job["payload"])
        if job.get("image_source_slug"):
            source_path = resolve_image_source(manifest, job["image_source_slug"])
            payload["input_reference"] = image_data_url(source_path)
        request_name = f"video_{slugify(payload['model'])}_{slugify(job['slug'])}.request.json"
        create_name = f"video_{slugify(payload['model'])}_{slugify(job['slug'])}.create.json"
        status_name = f"video_{slugify(payload['model'])}_{slugify(job['slug'])}.status.json"
        save_json(REQUESTS_DIR / request_name, payload)
        created = json_request("POST", f"{BASE_URL}/v1/videos", payload)
        save_json(RESPONSES_DIR / create_name, created)
        video_id = created.get("id")
        if not isinstance(video_id, str) or not video_id:
            raise RuntimeError(f"Unexpected video create response: {json.dumps(created, ensure_ascii=False)}")
        status = poll_video(video_id)
        save_json(RESPONSES_DIR / status_name, status)
        data, content_type = binary_request("GET", f"{BASE_URL}/v1/videos/{video_id}/content")
        ext = infer_ext_from_content_type(content_type)
        if ext == ".bin":
            ext = ".mp4"
        media_name = file_name("video", payload["model"], job["slug"], 1, ext)
        media_path = MEDIA_DIR / media_name
        write_bytes(media_path, data)
        videos_manifest.append(
            {
                "slug": job["slug"],
                "model": payload["model"],
                "video_id": video_id,
                "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
                "create_response": str((RESPONSES_DIR / create_name).relative_to(ROOT)),
                "status_response": str((RESPONSES_DIR / status_name).relative_to(ROOT)),
                "output": str(media_path.relative_to(ROOT)),
            }
        )
        print(f"Generated video via {payload['model']}: {media_path.relative_to(ROOT)}", flush=True)
    manifest["videos"] = merge_entries(manifest.get("videos", []), videos_manifest)


def generate_audio(manifest: dict[str, Any]) -> None:
    audio_manifest = []
    for job in filter_jobs("AIHUBMIX_AUDIO_SLUGS", AUDIO_JOBS):
        payload = job["payload"]
        model = payload["model"]
        request_name = f"audio_{slugify(model)}_{slugify(job['slug'])}.request.json"
        headers_name = f"audio_{slugify(model)}_{slugify(job['slug'])}.response-meta.json"
        save_json(REQUESTS_DIR / request_name, payload)
        data, content_type = binary_request("POST", f"{BASE_URL}/v1/audio/speech", payload)
        ext = infer_ext_from_content_type(content_type)
        if ext in {".bin", ".json"}:
            ext = infer_ext_from_bytes(data)
        media_name = file_name("audio", model, job["slug"], 1, ext)
        media_path = MEDIA_DIR / media_name
        write_bytes(media_path, data)
        save_json(
            RESPONSES_DIR / headers_name,
            {
                "model": model,
                "slug": job["slug"],
                "content_type": content_type,
                "detected_extension": ext,
                "bytes": len(data),
            },
        )
        audio_manifest.append(
            {
                "slug": job["slug"],
                "model": model,
                "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
                "response_meta": str((RESPONSES_DIR / headers_name).relative_to(ROOT)),
                "output": str(media_path.relative_to(ROOT)),
            }
        )
        print(f"Generated audio via {model}: {media_path.relative_to(ROOT)}", flush=True)
    manifest["audio"] = merge_entries(manifest.get("audio", []), audio_manifest)


def main() -> int:
    if not API_KEY:
        print("Set AIHUBMIX_API_KEY before running this script.", file=sys.stderr)
        return 1
    ensure_dirs()
    manifest = load_manifest()
    manifest["generated_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    failures = []
    stages = selected_stage_names()
    for runner, label in (
        (generate_images, "images"),
        (generate_videos, "videos"),
        (generate_audio, "audio"),
    ):
        if label not in stages:
            continue
        try:
            runner(manifest)
            manifest["errors"] = [item for item in manifest.get("errors", []) if item.get("stage") != label]
        except Exception as exc:  # noqa: BLE001
            failures.append({"stage": label, "error": str(exc)})
            manifest.setdefault("errors", []).append({"stage": label, "error": str(exc)})
            print(f"{label} failed: {exc}", file=sys.stderr, flush=True)
    save_json(MANIFEST_PATH, manifest)
    if failures:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
