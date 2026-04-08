#!/usr/bin/env python3
import json
import mimetypes
import os
import re
import signal
import sys
import time
import uuid
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen

from generate_aihubmix_media import (
    API_KEY,
    BASE_URL,
    ROOT,
    binary_request,
    extract_media_refs,
    fetch_url,
    file_name,
    image_data_url,
    infer_ext_from_bytes,
    infer_ext_from_content_type,
    save_json,
    slugify,
)


OUT_ROOT = ROOT / "generated" / "aihubmix_all_models_raw"
REQUESTS_DIR = OUT_ROOT / "requests"
RESPONSES_DIR = OUT_ROOT / "responses"
MEDIA_DIR = OUT_ROOT / "media"
MANIFEST_PATH = OUT_ROOT / "manifest.json"

SEED_IMAGE_PATH = ROOT / "tests" / "fixtures" / "ai_gptimage_1_5.png"
SEED_VIDEO_PATH = ROOT / "tests" / "fixtures" / "ai_wan.mp4"
SEED_VIDEO_URL = "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4"

OPENAI_IMAGE_MODELS = {
    "gpt-image-1.5",
    "gpt-image-1-mini",
    "gpt-image-1",
    "dall-e-2",
}
OPENAI_CHAT_IMAGE_MODELS = {
    "web-gpt-image-1.5",
    "gpt-4o-image-vip",
    "gpt-4o-image",
}
GOOGLE_IMAGEN_MODELS = {
    "imagen-4.0-ultra-generate-001",
    "imagen-4.0-generate-001",
    "imagen-4.0-fast-generate-preview-06-06",
    "imagen-4.0-fast-generate-001",
    "imagen-3.0-generate-002",
}
GEMINI_IMAGE_MODELS = {
    "gemini-3.1-flash-image-preview",
    "gemini-3.1-flash-image-preview-free",
    "gemini-3-pro-image-preview",
    "gemini-2.5-flash-image",
    "gemini-2.5-flash-image-preview",
    "gemini-2.0-flash-preview-image-generation",
}
GEMINI_NATIVE_MODELS = {
    "gemini-2.0-flash-exp",
    "gemini-2.0-pro-exp-02-05",
}
GOOGLE_IMAGEN_NOPREFIX_MODELS = {
    "imagen-4.0",
    "imagen-4.0-ultra",
    "imagen-4.0-generate-preview-05-20",
    "imagen-4.0-ultra-generate-exp-05-20",
}
CHAT_IMAGE_MODELS = {
    "Stable-Diffusion-3-5-Large",
    "wan2.7-image",
    "wan2.7-image-pro",
}
CHAT_VIDEO_MODELS = {
    "veo3",
    "veo-3",
    "veo3.1",
    "veo-3.0-generate-preview",
    "veo-3.1-fast-generate-preview",
    "veo-3.1-generate-preview",
    "web-sora-2",
    "web-sora-2-pro",
}
MULTIPART_VIDEO_MODELS = {
    "wan2.7-i2v",
    "wan2.7-r2v",
    "wan2.7-videoedit",
}
DOUBAO_IMAGE_MODELS = {
    "doubao-seedream-5.0-lite",
    "doubao-seedream-4-5",
    "doubao-seedream-4-0",
}
BFL_IMAGE_MODELS = {
    "FLUX-1.1-pro",
}
BFL_ASYNC_MODELS = {
    "flux-2-flex",
    "flux-2-pro",
}
IDEOGRAM_LEGACY_MODELS = {
    "V_2",
    "V_2_TURBO",
    "V_2A",
    "V_2A_TURBO",
    "V_1",
    "V_1_TURBO",
}
VIDEO_IMAGE_MODELS = {
    "doubao-seedance-2-0-260128",
    "doubao-seedance-2-0-fast-260128",
    "wan2.7-i2v",
    "doubao-seedance-1-5-pro-251215",
    "doubao-seedance-1-0-pro-250528",
    "doubao-seedance-1-0-pro-fast-251015",
    "wan2.6-i2v",
    "jimeng-3.0-1080p",
    "jimeng-3.0-720p",
    "jimeng-3.0-pro",
    "wan2.2-i2v-plus",
    "wan2.5-i2v-preview",
}
VIDEO_VIDEO_MODELS = {
    "wan2.7-r2v",
    "wan2.7-videoedit",
}
VIDEO_EIGHT_SECOND_MODELS = {
    "veo3.1",
    "veo-3.0-generate-preview",
    "veo-3.1-fast-generate-preview",
    "veo-3.1-generate-preview",
    "veo-3",
    "veo3",
}
SUPPORTED_STAGES = {"images", "videos", "tts"}


class RequestTimedOut(RuntimeError):
    pass


def ensure_dirs() -> None:
    for path in (REQUESTS_DIR, RESPONSES_DIR, MEDIA_DIR):
        path.mkdir(parents=True, exist_ok=True)


def run_with_alarm(timeout: int, fn):
    if timeout <= 0:
        return fn()

    def handler(_signum, _frame):
        raise RequestTimedOut(f"Request timed out after {timeout}s")

    previous = signal.signal(signal.SIGALRM, handler)
    signal.setitimer(signal.ITIMER_REAL, timeout)
    try:
        return fn()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)


def selected_stage_names() -> set[str]:
    raw = os.environ.get("AIHUBMIX_STAGES", "images,videos,tts")
    stages = {part.strip() for part in raw.split(",") if part.strip()}
    return stages & SUPPORTED_STAGES


def filter_model_ids(env_name: str, models: list[dict[str, Any]]) -> list[dict[str, Any]]:
    raw = os.environ.get(env_name, "").strip()
    if not raw:
        return models
    allowed = {part.strip() for part in raw.split(",") if part.strip()}
    return [model for model in models if model["model_id"] in allowed]


def json_api(
    method: str,
    url: str,
    payload: dict[str, Any] | None = None,
    headers: dict[str, str] | None = None,
    timeout: int = 1800,
) -> dict[str, Any]:
    req_headers = {"Authorization": f"Bearer {API_KEY}"}
    if payload is not None:
        req_headers["Content-Type"] = "application/json"
    if headers:
        req_headers.update(headers)
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = Request(url, data=data, method=method, headers=req_headers)
    try:
        def do_request():
            with urlopen(request, timeout=timeout) as response:
                return json.loads(response.read().decode("utf-8"))

        return run_with_alarm(timeout + 5, do_request)
    except HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {url} failed with {exc.code}: {detail}") from exc
    except URLError as exc:
        raise RuntimeError(f"{method} {url} failed: {exc}") from exc


def multipart_json_api(
    url: str,
    fields: dict[str, Any],
    files: list[tuple[str, Path]] | None = None,
    headers: dict[str, str] | None = None,
) -> dict[str, Any]:
    boundary = f"----AIHubMixBoundary{uuid.uuid4().hex}"
    body = bytearray()
    for key, value in fields.items():
        body.extend(f"--{boundary}\r\n".encode("utf-8"))
        body.extend(f'Content-Disposition: form-data; name="{key}"\r\n\r\n'.encode("utf-8"))
        body.extend(str(value).encode("utf-8"))
        body.extend(b"\r\n")
    for field_name, file_path in files or []:
        mime = mimetypes.guess_type(file_path.name)[0] or "application/octet-stream"
        body.extend(f"--{boundary}\r\n".encode("utf-8"))
        body.extend(
            f'Content-Disposition: form-data; name="{field_name}"; filename="{file_path.name}"\r\n'.encode("utf-8")
        )
        body.extend(f"Content-Type: {mime}\r\n\r\n".encode("utf-8"))
        body.extend(file_path.read_bytes())
        body.extend(b"\r\n")
    body.extend(f"--{boundary}--\r\n".encode("utf-8"))
    req_headers = {"Api-Key": API_KEY, "Content-Type": f"multipart/form-data; boundary={boundary}"}
    if headers:
        req_headers.update(headers)
    request = Request(url, data=bytes(body), method="POST", headers=req_headers)
    try:
        def do_request():
            with urlopen(request, timeout=300) as response:
                return json.loads(response.read().decode("utf-8"))

        return run_with_alarm(305, do_request)
    except HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"POST {url} failed with {exc.code}: {detail}") from exc
    except URLError as exc:
        raise RuntimeError(f"POST {url} failed: {exc}") from exc


def list_models(model_type: str) -> list[dict[str, Any]]:
    url = f"{BASE_URL}/api/v1/models?type={quote(model_type)}&sort_by=order&sort_order=asc"
    return json_api("GET", url).get("data", [])


def save_media_from_refs(prefix: str, model: str, slug: str, refs: list[dict[str, str]]) -> list[str]:
    outputs = []
    for index, ref in enumerate(refs, start=1):
        if ref["kind"] == "url":
            data, content_type = fetch_url(ref["value"])
            ext = infer_ext_from_content_type(content_type)
        elif ref["kind"] == "data_url":
            header, raw = ref["value"].split(",", 1)
            data = __import__("base64").b64decode(raw)
            ext = infer_ext_from_content_type(header.split(";")[0][5:])
        else:
            data = __import__("base64").b64decode(ref["value"])
            ext = infer_ext_from_content_type(ref.get("mime_type")) if ref.get("mime_type") else infer_ext_from_bytes(data)
            if ext in {".bin", ".json"}:
                ext = infer_ext_from_bytes(data)
        if ext in {".bin", ".json"}:
            ext = infer_ext_from_bytes(data)
        media_name = file_name(prefix, model, slug, index, ext)
        media_path = MEDIA_DIR / media_name
        media_path.write_bytes(data)
        outputs.append(str(media_path.relative_to(ROOT)))
    return outputs


def extract_gemini_chat_refs(response: dict[str, Any]) -> list[dict[str, str]]:
    refs: list[dict[str, str]] = []
    for choice in response.get("choices", []):
        message = choice.get("message", {})
        for item in message.get("multi_mod_content", []):
            inline_data = item.get("inline_data")
            if isinstance(inline_data, dict) and inline_data.get("data"):
                ref = {"kind": "base64", "value": inline_data["data"]}
                if isinstance(inline_data.get("mime_type"), str):
                    ref["mime_type"] = inline_data["mime_type"]
                refs.append(ref)
    return refs


def extract_embedded_url_refs(response: dict[str, Any]) -> list[dict[str, str]]:
    refs: list[dict[str, str]] = []

    def walk(value: Any) -> None:
        if isinstance(value, dict):
            for nested in value.values():
                walk(nested)
            return
        if isinstance(value, list):
            for item in value:
                walk(item)
            return
        if isinstance(value, str):
            for match in re.findall(r"https?://[^\s)>\"]+", value):
                refs.append({"kind": "url", "value": match})

    walk(response)
    unique = []
    seen = set()
    for ref in refs:
        marker = ref["value"]
        if marker not in seen:
            unique.append(ref)
            seen.add(marker)
    return unique


def openai_image_payload(prompt: str, model_id: str) -> dict[str, Any]:
    payload = {"input": {"prompt": prompt, "size": "1024x1024", "n": 1}}
    if model_id not in {"dall-e-2", "dall-e-3"}:
        payload["input"]["output_format"] = "png"
    if model_id.startswith("gpt-image-1"):
        payload["input"]["quality"] = "medium"
    return payload


def google_imagen_payload(prompt: str) -> dict[str, Any]:
    return {"input": {"prompt": prompt, "numberOfImages": 1}}


def doubao_payload(prompt: str) -> dict[str, Any]:
    return {
        "input": {
            "prompt": prompt,
            "size": "2K",
            "sequential_image_generation": "disabled",
            "response_format": "url",
            "watermark": True,
        }
    }


def qwen_payload(prompt: str) -> dict[str, Any]:
    return {"input": {"prompt": prompt, "n": 1, "size": "1024x1024"}}


def qwen_edit_payload(prompt: str) -> dict[str, Any]:
    return {"input": {"prompt": prompt, "image": image_data_url(SEED_IMAGE_PATH), "n": 1, "size": "1024x1024"}}


def flux_payload(prompt: str) -> dict[str, Any]:
    return {"input": {"prompt": prompt, "aspect_ratio": "1:1"}}


def musesteamer_payload(prompt: str) -> dict[str, Any]:
    return {
        "input": {
            "prompt": prompt,
            "n": 1,
            "size": "1024x1024",
            "guidance": 7.5,
            "watermark": False,
        }
    }


def generic_payload(prompt: str) -> dict[str, Any]:
    return {"input": {"prompt": prompt}}


def image_prompt(model_id: str) -> str:
    if "poster" in model_id.lower() or model_id in IDEOGRAM_LEGACY_MODELS | {"V3"}:
        return "A bold editorial travel poster for a moonlit coastal city, elegant typography space, clean composition, vivid but controlled color, premium print design"
    if "edit" in model_id.lower() or "kontext" in model_id.lower():
        return "Transform the source image into a misty dawn scene while preserving the courtyard layout and architecture, realistic light, no text"
    if model_id.startswith("gemini-"):
        return "Generate a cinematic still image of a red kite gliding above a bright blue sea at golden hour, realistic lighting, clean sky, no text."
    if model_id.startswith("imagen-"):
        return "A dreamlike underwater library carved into coral, glowing bookshelves, shafts of sunlight through the water, whimsical environment concept art"
    if model_id.startswith("doubao-"):
        return "A quiet tea house courtyard after rain, polished stone, bamboo shadows, rising steam, ultra-detailed realistic photography"
    if model_id.startswith("flux") or model_id.startswith("FLUX"):
        return "A sculptural still life of chrome fruit on a linen cloth, gallery lighting, crisp reflections, premium art photography"
    if model_id.startswith("dall-e") or model_id.startswith("gpt-") or model_id.startswith("web-gpt"):
        return "A photoreal rainy night market with lantern reflections on wet pavement, candid street photography, layered depth, no visible text"
    if model_id.startswith("wan"):
        return "A serene mountain lake at sunrise with drifting mist and tall reeds, painterly but grounded, soft atmosphere"
    if model_id.startswith("musesteamer"):
        return "A tiny red kite over a blue sea with clean composition and bright daylight"
    return "A cinematic landscape with strong composition, high detail, natural light, and no text"


def run_prediction_image(model_id: str, provider_candidates: list[str], payload: dict[str, Any], slug: str, timeout: int = 120) -> tuple[dict[str, Any], list[str]]:
    last_error = None
    for provider in provider_candidates:
        request_name = f"image_{slugify(provider)}-{slugify(model_id)}_{slug}.request.json"
        response_name = f"image_{slugify(provider)}-{slugify(model_id)}_{slug}.response.json"
        save_json(REQUESTS_DIR / request_name, payload)
        try:
            response = json_api("POST", f"{BASE_URL}/v1/models/{provider}/{model_id}/predictions", payload, timeout=timeout)
            save_json(RESPONSES_DIR / response_name, response)
            refs = extract_media_refs(response)
            if not refs:
                raise RuntimeError("No image output references found")
            outputs = save_media_from_refs("image", f"{provider}-{model_id}", slug, refs)
            return (
                {
                    "model_id": model_id,
                    "provider": provider,
                    "strategy": "predictions",
                    "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
                    "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
                },
                outputs,
            )
        except Exception as exc:  # noqa: BLE001
            last_error = str(exc)
    raise RuntimeError(last_error or f"No provider worked for {model_id}")


def run_gemini_chat_image(model_id: str, slug: str) -> tuple[dict[str, Any], list[str]]:
    payload = {
        "model": model_id,
        "messages": [{"role": "user", "content": image_prompt(model_id)}],
        "modalities": ["text", "image"],
    }
    request_name = f"image_chat-{slugify(model_id)}_{slug}.request.json"
    response_name = f"image_chat-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    response = json_api("POST", f"{BASE_URL}/v1/chat/completions", payload, timeout=300)
    save_json(RESPONSES_DIR / response_name, response)
    refs = extract_gemini_chat_refs(response)
    if not refs:
        raise RuntimeError(f"No image output references found for {model_id}")
    outputs = save_media_from_refs("image", model_id, slug, refs)
    return (
        {
            "model_id": model_id,
            "strategy": "chat_completions",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_openai_chat_image(model_id: str, slug: str) -> tuple[dict[str, Any], list[str]]:
    payload = {
        "model": model_id,
        "messages": [{"role": "user", "content": image_prompt(model_id)}],
        "modalities": ["text", "image"],
    }
    request_name = f"image_chat-{slugify(model_id)}_{slug}.request.json"
    response_name = f"image_chat-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    response = json_api("POST", f"{BASE_URL}/v1/chat/completions", payload, timeout=300)
    save_json(RESPONSES_DIR / response_name, response)
    refs = extract_embedded_url_refs(response)
    if not refs:
        raise RuntimeError(f"No image output references found for {model_id}")
    outputs = save_media_from_refs("image", model_id, slug, refs)
    return (
        {
            "model_id": model_id,
            "strategy": "chat_completions",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_ideogram_v3(slug: str) -> tuple[dict[str, Any], list[str]]:
    fields = {
        "prompt": image_prompt("V3"),
        "rendering_speed": "DEFAULT",
        "num_images": 1,
        "aspect_ratio": "1x1",
        "magic_prompt": "AUTO",
        "style_type": "AUTO",
    }
    request_name = f"image_ideogram-v3_{slug}.request.json"
    response_name = f"image_ideogram-v3_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, fields)
    response = multipart_json_api(f"{BASE_URL}/ideogram/v1/ideogram-v3/generate", fields)
    save_json(RESPONSES_DIR / response_name, response)
    refs = extract_media_refs(response)
    if not refs:
        raise RuntimeError("No image output references found for Ideogram V3")
    outputs = save_media_from_refs("image", "ideogram-V3", slug, refs)
    return (
        {
            "model_id": "V3",
            "strategy": "ideogram_v3_generate",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_ideogram_legacy(model_id: str, slug: str) -> tuple[dict[str, Any], list[str]]:
    payload = {
        "image_request": {
            "prompt": image_prompt(model_id),
            "aspect_ratio": "ASPECT_1_1",
            "model": model_id,
            "magic_prompt_option": "AUTO",
        }
    }
    request_name = f"image_ideogram-{slugify(model_id)}_{slug}.request.json"
    response_name = f"image_ideogram-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    response = json_api(
        "POST",
        f"{BASE_URL}/ideogram/generate",
        payload,
        headers={"Api-Key": API_KEY},
        timeout=300,
    )
    save_json(RESPONSES_DIR / response_name, response)
    refs = extract_media_refs(response)
    if not refs:
        raise RuntimeError(f"No image output references found for {model_id}")
    outputs = save_media_from_refs("image", f"ideogram-{model_id}", slug, refs)
    return (
        {
            "model_id": model_id,
            "strategy": "ideogram_generate",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_ideogram_upscale(slug: str) -> tuple[dict[str, Any], list[str]]:
    fields = {"image_request": "{}"}
    request_name = f"image_ideogram-upscale_{slug}.request.json"
    response_name = f"image_ideogram-upscale_{slug}.response.json"
    save_json(
        REQUESTS_DIR / request_name,
        {"seed_image": str(SEED_IMAGE_PATH.relative_to(ROOT))},
    )
    response = multipart_json_api(
        f"{BASE_URL}/ideogram/upscale",
        fields,
        files=[("image_file", SEED_IMAGE_PATH)],
    )
    save_json(RESPONSES_DIR / response_name, response)
    refs = extract_media_refs(response)
    if not refs:
        raise RuntimeError("No image output references found for UPSCALE")
    outputs = save_media_from_refs("image", "ideogram-UPSCALE", slug, refs)
    return (
        {
            "model_id": "UPSCALE",
            "strategy": "ideogram_upscale",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_ideogram_describe(slug: str) -> tuple[dict[str, Any], list[str]]:
    fields = {}
    request_name = f"image_ideogram-describe_{slug}.request.json"
    response_name = f"image_ideogram-describe_{slug}.response.json"
    save_json(
        REQUESTS_DIR / request_name,
        {"seed_image": str(SEED_IMAGE_PATH.relative_to(ROOT))},
    )
    response = multipart_json_api(
        f"{BASE_URL}/ideogram/describe",
        fields,
        files=[("image_file", SEED_IMAGE_PATH)],
    )
    save_json(RESPONSES_DIR / response_name, response)
    return (
        {
            "model_id": "DESCRIBE",
            "strategy": "ideogram_describe",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
            "notes": "This model returns text descriptions only and does not output media files.",
        },
        [],
    )


def poll_flux_prediction(model_id: str, slug: str, payload: dict[str, Any]) -> tuple[dict[str, Any], list[str]]:
    """Submit a BFL prediction and poll until the async task is ready."""
    request_name = f"image_bfl-{slugify(model_id)}_{slug}.request.json"
    response_name = f"image_bfl-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    submit_response = json_api("POST", f"{BASE_URL}/v1/models/bfl/{model_id}/predictions", payload, timeout=120)
    save_json(RESPONSES_DIR / response_name, submit_response)
    # Check if media refs are already present (synchronous response)
    refs = extract_media_refs(submit_response)
    if refs:
        outputs = save_media_from_refs("image", f"bfl-{model_id}", slug, refs)
        return (
            {
                "model_id": model_id,
                "provider": "bfl",
                "strategy": "predictions_poll",
                "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
                "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
            },
            outputs,
        )
    # Async: extract taskId and poll
    task_id = submit_response.get("taskId") or submit_response.get("task_id") or submit_response.get("id")
    if not task_id:
        raise RuntimeError(f"BFL prediction returned no taskId: {json.dumps(submit_response, ensure_ascii=False)[:500]}")
    poll_name = f"image_bfl-{slugify(model_id)}_{slug}.poll.json"
    deadline = time.time() + 600  # 10 minutes
    while True:
        poll_response = json_api("GET", f"{BASE_URL}/v1/tasks/{task_id}", timeout=60)
        save_json(RESPONSES_DIR / poll_name, poll_response)
        status = (poll_response.get("status") or "").lower()
        if status in {"ready", "succeeded", "completed"}:
            break
        if status in {"failed", "error"}:
            raise RuntimeError(f"BFL task failed: {json.dumps(poll_response, ensure_ascii=False)[:500]}")
        if time.time() > deadline:
            raise RuntimeError(f"BFL polling timed out for {model_id} task {task_id}")
        time.sleep(10)
    # Extract result
    result = poll_response.get("result", {})
    sample_url = result.get("sample") or result.get("url")
    if sample_url:
        refs = [{"kind": "url", "value": sample_url}]
    else:
        refs = extract_media_refs(poll_response)
    if not refs:
        raise RuntimeError(f"No image output from BFL poll: {json.dumps(poll_response, ensure_ascii=False)[:500]}")
    outputs = save_media_from_refs("image", f"bfl-{model_id}", slug, refs)
    return (
        {
            "model_id": model_id,
            "provider": "bfl",
            "strategy": "predictions_poll",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_images_generations(model_id: str, slug: str, extra_fields: dict[str, Any] | None = None) -> tuple[dict[str, Any], list[str]]:
    """Use /v1/images/generations endpoint (e.g. for FLUX.1-Kontext-pro)."""
    payload: dict[str, Any] = {
        "model": model_id,
        "prompt": image_prompt(model_id),
    }
    if extra_fields:
        payload.update(extra_fields)
    request_name = f"image_gen-{slugify(model_id)}_{slug}.request.json"
    response_name = f"image_gen-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    response = json_api("POST", f"{BASE_URL}/v1/images/generations", payload, timeout=300)
    save_json(RESPONSES_DIR / response_name, response)
    refs = extract_media_refs(response)
    # Also check data[].b64_json explicitly
    for item in response.get("data", []):
        b64 = item.get("b64_json")
        if b64 and ("base64", b64) not in {(r["kind"], r["value"]) for r in refs}:
            refs.append({"kind": "base64", "value": b64, "mime_type": "image/png"})
    if not refs:
        raise RuntimeError(f"No image output from /v1/images/generations for {model_id}")
    outputs = save_media_from_refs("image", model_id, slug, refs)
    return (
        {
            "model_id": model_id,
            "strategy": "images_generations",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_chat_image(model_id: str, slug: str) -> tuple[dict[str, Any], list[str]]:
    """Generate images via /v1/chat/completions (SD-3-5-Large, wan2.7-image, etc.)."""
    payload = {
        "model": model_id,
        "messages": [{"role": "user", "content": image_prompt(model_id)}],
    }
    request_name = f"image_chat-{slugify(model_id)}_{slug}.request.json"
    response_name = f"image_chat-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    response = json_api("POST", f"{BASE_URL}/v1/chat/completions", payload, timeout=300)
    save_json(RESPONSES_DIR / response_name, response)
    refs = extract_media_refs(response)
    if not refs:
        raise RuntimeError(f"No image output from chat completions for {model_id}")
    outputs = save_media_from_refs("image", model_id, slug, refs)
    return (
        {
            "model_id": model_id,
            "strategy": "chat_completions",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_gemini_native_image(model_id: str, slug: str) -> tuple[dict[str, Any], list[str]]:
    """Generate images via Gemini native API (/gemini/v1beta/models/{model}:generateContent)."""
    payload = {
        "contents": [
            {
                "parts": [{"text": image_prompt(model_id)}],
                "role": "user",
            }
        ],
        "generationConfig": {"responseModalities": ["TEXT", "IMAGE"]},
    }
    request_name = f"image_gemini-native-{slugify(model_id)}_{slug}.request.json"
    response_name = f"image_gemini-native-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    # Gemini native API uses x-goog-api-key header, NOT Bearer token
    url = f"{BASE_URL}/gemini/v1beta/models/{model_id}:generateContent"
    data = json.dumps(payload).encode("utf-8")
    request_obj = Request(
        url,
        data=data,
        method="POST",
        headers={
            "x-goog-api-key": API_KEY,
            "Content-Type": "application/json",
        },
    )
    try:
        def do_request():
            with urlopen(request_obj, timeout=300) as resp:
                return json.loads(resp.read().decode("utf-8"))

        response = run_with_alarm(305, do_request)
    except HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"POST {url} failed with {exc.code}: {detail}") from exc
    except URLError as exc:
        raise RuntimeError(f"POST {url} failed: {exc}") from exc
    save_json(RESPONSES_DIR / response_name, response)
    # Extract inline image data: candidates[].content.parts[].inlineData.{mimeType, data}
    refs: list[dict[str, str]] = []
    for candidate in response.get("candidates", []):
        for part in candidate.get("content", {}).get("parts", []):
            inline = part.get("inlineData")
            if inline and inline.get("data"):
                refs.append({
                    "kind": "base64",
                    "value": inline["data"],
                    "mime_type": inline.get("mimeType", "image/png"),
                })
    if not refs:
        # Fallback: try generic extraction
        refs = extract_media_refs(response)
    if not refs:
        raise RuntimeError(f"No image output from Gemini native API for {model_id}")
    outputs = save_media_from_refs("image", model_id, slug, refs)
    return (
        {
            "model_id": model_id,
            "strategy": "gemini_native",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_prediction_image_noprefix(model_id: str, payload: dict[str, Any], slug: str) -> tuple[dict[str, Any], list[str]]:
    """Run predictions without a provider prefix: /v1/models/{model}/predictions."""
    request_name = f"image_noprefix-{slugify(model_id)}_{slug}.request.json"
    response_name = f"image_noprefix-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    response = json_api("POST", f"{BASE_URL}/v1/models/{model_id}/predictions", payload, timeout=120)
    save_json(RESPONSES_DIR / response_name, response)
    refs = extract_media_refs(response)
    if not refs:
        raise RuntimeError(f"No image output references found for {model_id}")
    outputs = save_media_from_refs("image", model_id, slug, refs)
    return (
        {
            "model_id": model_id,
            "strategy": "predictions_no_prefix",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_chat_video(model_id: str, slug: str) -> tuple[dict[str, Any], list[str]]:
    """Generate video via /v1/chat/completions (veo, web-sora models)."""
    payload = {
        "model": model_id,
        "messages": [{"role": "user", "content": video_prompt(model_id)}],
    }
    request_name = f"video_chat-{slugify(model_id)}_{slug}.request.json"
    response_name = f"video_chat-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    response = json_api("POST", f"{BASE_URL}/v1/chat/completions", payload, timeout=600)
    save_json(RESPONSES_DIR / response_name, response)
    refs = extract_media_refs(response)
    if not refs:
        raise RuntimeError(f"No video output from chat completions for {model_id}")
    outputs = save_media_from_refs("video", model_id, slug, refs)
    return (
        {
            "model_id": model_id,
            "strategy": "chat_completions",
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_multipart_video(model_id: str, slug: str) -> tuple[dict[str, Any], list[str]]:
    """Submit video via multipart/form-data POST to /v1/videos and poll."""
    if model_id in VIDEO_VIDEO_MODELS:
        file_path = SEED_VIDEO_PATH
    else:
        file_path = SEED_IMAGE_PATH
    fields = {
        "model": model_id,
        "prompt": video_prompt(model_id),
        "size": "1280x720",
        "seconds": video_seconds(model_id),
    }
    request_name = f"video_multipart-{slugify(model_id)}_{slug}.request.json"
    create_name = f"video_multipart-{slugify(model_id)}_{slug}.create.json"
    status_name = f"video_multipart-{slugify(model_id)}_{slug}.status.json"
    save_json(REQUESTS_DIR / request_name, {**fields, "input_reference": str(file_path.relative_to(ROOT))})
    # Build multipart body
    boundary = f"----AIHubMixBoundary{uuid.uuid4().hex}"
    body = bytearray()
    for key, value in fields.items():
        body.extend(f"--{boundary}\r\n".encode("utf-8"))
        body.extend(f'Content-Disposition: form-data; name="{key}"\r\n\r\n'.encode("utf-8"))
        body.extend(str(value).encode("utf-8"))
        body.extend(b"\r\n")
    # File part
    mime = mimetypes.guess_type(file_path.name)[0] or "application/octet-stream"
    body.extend(f"--{boundary}\r\n".encode("utf-8"))
    body.extend(
        f'Content-Disposition: form-data; name="input_reference"; filename="{file_path.name}"\r\n'.encode("utf-8")
    )
    body.extend(f"Content-Type: {mime}\r\n\r\n".encode("utf-8"))
    body.extend(file_path.read_bytes())
    body.extend(b"\r\n")
    body.extend(f"--{boundary}--\r\n".encode("utf-8"))
    url = f"{BASE_URL}/v1/videos"
    request_obj = Request(
        url,
        data=bytes(body),
        method="POST",
        headers={
            "Authorization": f"Bearer {API_KEY}",
            "Content-Type": f"multipart/form-data; boundary={boundary}",
        },
    )
    try:
        def do_request():
            with urlopen(request_obj, timeout=600) as resp:
                return json.loads(resp.read().decode("utf-8"))

        created_response = run_with_alarm(605, do_request)
    except HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"POST {url} failed with {exc.code}: {detail}") from exc
    except URLError as exc:
        raise RuntimeError(f"POST {url} failed: {exc}") from exc
    save_json(RESPONSES_DIR / create_name, created_response)
    video_id = created_response.get("id")
    if not isinstance(video_id, str) or not video_id:
        raise RuntimeError(f"Unexpected create response: {json.dumps(created_response, ensure_ascii=False)}")
    # Poll until complete
    deadline = time.time() + 60 * 30
    while True:
        status_resp = json_api("GET", f"{BASE_URL}/v1/videos/{video_id}", timeout=60)
        save_json(RESPONSES_DIR / status_name, status_resp)
        current = status_resp.get("status")
        if current == "completed":
            break
        if current == "failed" or status_resp.get("type") == "error":
            raise RuntimeError(f"Multipart video job failed for {model_id}: {json.dumps(status_resp, ensure_ascii=False)}")
        if time.time() > deadline:
            raise RuntimeError(f"Timed out waiting for multipart video job {model_id}")
        time.sleep(15)
    # Download content
    data, content_type = binary_request("GET", f"{BASE_URL}/v1/videos/{video_id}/content")
    ext = infer_ext_from_content_type(content_type)
    if ext == ".bin":
        ext = ".mp4"
    media_name = file_name("video", model_id, slug, 1, ext)
    media_path = MEDIA_DIR / media_name
    media_path.write_bytes(data)
    outputs = [str(media_path.relative_to(ROOT))]
    return (
        {
            "model_id": model_id,
            "strategy": "multipart_videos",
            "video_id": video_id,
            "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
            "create_response": str((RESPONSES_DIR / create_name).relative_to(ROOT)),
            "status_response": str((RESPONSES_DIR / status_name).relative_to(ROOT)),
        },
        outputs,
    )


def run_image_model(model: dict[str, Any]) -> dict[str, Any]:
    model_id = model["model_id"]
    slug = "coverage"
    # --- Fix 6: Gemini native API for older exp models ---
    if model_id in GEMINI_NATIVE_MODELS:
        base_entry, outputs = run_gemini_native_image(model_id, slug)
    # --- Standard Gemini chat completions ---
    elif model_id in GEMINI_IMAGE_MODELS:
        base_entry, outputs = run_gemini_chat_image(model_id, slug)
    # --- Fix 3: Google Imagen short aliases (no provider prefix) ---
    elif model_id in GOOGLE_IMAGEN_NOPREFIX_MODELS:
        base_entry, outputs = run_prediction_image_noprefix(model_id, google_imagen_payload(image_prompt(model_id)), slug)
    # --- Google Imagen models that use google/ provider prefix ---
    elif model_id in GOOGLE_IMAGEN_MODELS:
        base_entry, outputs = run_prediction_image(model_id, ["google"], google_imagen_payload(image_prompt(model_id)), slug)
    # --- OpenAI image models ---
    elif model_id in OPENAI_IMAGE_MODELS:
        base_entry, outputs = run_prediction_image(model_id, ["openai"], openai_image_payload(image_prompt(model_id), model_id), slug)
    elif model_id in OPENAI_CHAT_IMAGE_MODELS:
        base_entry, outputs = run_openai_chat_image(model_id, slug)
    # --- Doubao image models ---
    elif model_id in DOUBAO_IMAGE_MODELS:
        base_entry, outputs = run_prediction_image(model_id, ["doubao"], doubao_payload(image_prompt(model_id)), slug)
    # --- Qwen image models ---
    elif model_id == "qwen-image":
        base_entry, outputs = run_prediction_image(model_id, ["qianfan"], qwen_payload(image_prompt(model_id)), slug)
    elif model_id == "qwen-image-edit":
        base_entry, outputs = run_prediction_image(model_id, ["qianfan"], qwen_edit_payload(image_prompt(model_id)), slug)
    # --- Fix 2: FLUX.1-Kontext-pro via /v1/images/generations ---
    elif model_id == "FLUX.1-Kontext-pro":
        base_entry, outputs = run_images_generations(model_id, slug, extra_fields={"safety_tolerance": 6})
    # --- Fix 1: BFL Flux models with async polling ---
    elif model_id in BFL_ASYNC_MODELS:
        base_entry, outputs = poll_flux_prediction(model_id, slug, flux_payload(image_prompt(model_id)))
    # --- Other BFL models (synchronous) ---
    elif model_id in BFL_IMAGE_MODELS:
        base_entry, outputs = run_prediction_image(model_id, ["bfl"], flux_payload(image_prompt(model_id)), slug)
    # --- Ideogram models ---
    elif model_id == "V3":
        base_entry, outputs = run_ideogram_v3(slug)
    elif model_id in IDEOGRAM_LEGACY_MODELS:
        base_entry, outputs = run_ideogram_legacy(model_id, slug)
    elif model_id == "UPSCALE":
        base_entry, outputs = run_ideogram_upscale(slug)
    elif model_id == "DESCRIBE":
        base_entry, outputs = run_ideogram_describe(slug)
    # --- Fix 5: SD-3-5-Large and wan2.7-image via chat completions ---
    elif model_id in CHAT_IMAGE_MODELS:
        base_entry, outputs = run_chat_image(model_id, slug)
    # --- Fix 4: musesteamer-air-image via qianfan only with 300s timeout ---
    elif model_id == "musesteamer-air-image":
        base_entry, outputs = run_prediction_image(
            model_id,
            ["qianfan"],
            musesteamer_payload(image_prompt(model_id)),
            slug,
            timeout=300,
        )
    elif model_id == "dall-e-3":
        base_entry, outputs = run_prediction_image(model_id, ["openai"], openai_image_payload(image_prompt(model_id), model_id), slug)
    else:
        raise RuntimeError(f"No strategy configured for {model_id}")
    base_entry["outputs"] = outputs
    return base_entry


def video_prompt(model_id: str) -> str:
    if model_id in VIDEO_VIDEO_MODELS:
        return "Restyle the source video into a polished cinematic sequence with smoother motion, richer contrast, and natural pacing."
    if model_id in VIDEO_IMAGE_MODELS:
        return "Animate the source image with subtle camera movement, realistic wind in the leaves, drifting steam, and calm cinematic motion."
    if model_id in VIDEO_EIGHT_SECOND_MODELS or model_id.startswith("veo"):
        return "A cinematic drone glide over a coastal cliff town at sunrise, natural ocean ambience, seagulls in the distance, realistic motion."
    if "sora" in model_id:
        return "A hand-folded paper crane transforms into a real white bird and flies through an open studio window, cinematic light, graceful motion."
    return "A cinematic shot of lanterns swaying in a coastal alley after rain, gentle camera move, realistic detail and motion."


def video_seconds(model_id: str) -> str:
    if model_id in VIDEO_EIGHT_SECOND_MODELS:
        return "8"
    return "5"


def video_payload(model_id: str) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "model": model_id,
        "prompt": video_prompt(model_id),
        "seconds": video_seconds(model_id),
        "size": "1280x720",
    }
    if model_id in VIDEO_IMAGE_MODELS:
        payload["input_reference"] = image_data_url(SEED_IMAGE_PATH)
    elif model_id in VIDEO_VIDEO_MODELS:
        payload["input_reference"] = SEED_VIDEO_URL
    return payload


def poll_videos(pending: list[dict[str, Any]]) -> list[dict[str, Any]]:
    deadline = time.time() + 60 * 90
    waiting = pending[:]
    finished: list[dict[str, Any]] = []
    while waiting:
        next_waiting = []
        for item in waiting:
            status = json_api("GET", f"{BASE_URL}/v1/videos/{item['video_id']}")
            save_json(RESPONSES_DIR / item["status_file"], status)
            current = status.get("status")
            if current == "completed":
                item["status"] = status
                finished.append(item)
            elif current == "failed":
                raise RuntimeError(f"Video job failed for {item['model_id']}: {json.dumps(status, ensure_ascii=False)}")
            else:
                next_waiting.append(item)
        if next_waiting:
            if time.time() > deadline:
                raise RuntimeError("Timed out waiting for video jobs")
            print(f"Waiting on {len(next_waiting)} video jobs", flush=True)
            time.sleep(15)
        waiting = next_waiting
    return finished


def run_videos(manifest: dict[str, Any]) -> None:
    models = filter_model_ids("AIHUBMIX_VIDEO_IDS", list_models("video"))
    created: list[dict[str, Any]] = []
    successes: list[dict[str, Any]] = []
    failures: list[dict[str, Any]] = []
    slug = "coverage"

    # Split models into three strategies
    standard_models = []
    chat_models = []
    multipart_models = []
    for model in models:
        model_id = model["model_id"]
        if model_id in CHAT_VIDEO_MODELS:
            chat_models.append(model)
        elif model_id in MULTIPART_VIDEO_MODELS:
            multipart_models.append(model)
        else:
            standard_models.append(model)

    # --- Fix 9: Veo/web-sora via chat completions ---
    for model in chat_models:
        model_id = model["model_id"]
        print(f"Submitting video model {model_id} (chat completions)", flush=True)
        try:
            base_entry, outputs = run_chat_video(model_id, slug)
            base_entry["outputs"] = outputs
            successes.append(base_entry)
            print(f"Generated video via {model_id} (chat): {', '.join(outputs)}", flush=True)
        except Exception as exc:  # noqa: BLE001
            failures.append({"model_id": model_id, "error": str(exc)})
            print(f"Video chat failed for {model_id}: {exc}", file=sys.stderr, flush=True)

    # --- Fix 8: wan2.7-i2v/r2v/videoedit via multipart file upload ---
    for model in multipart_models:
        model_id = model["model_id"]
        print(f"Submitting video model {model_id} (multipart upload)", flush=True)
        try:
            base_entry, outputs = run_multipart_video(model_id, slug)
            base_entry["outputs"] = outputs
            successes.append(base_entry)
            print(f"Generated video via {model_id} (multipart): {', '.join(outputs)}", flush=True)
        except Exception as exc:  # noqa: BLE001
            failures.append({"model_id": model_id, "error": str(exc)})
            print(f"Video multipart failed for {model_id}: {exc}", file=sys.stderr, flush=True)

    # --- Standard /v1/videos API for remaining models ---
    for model in standard_models:
        model_id = model["model_id"]
        payload = video_payload(model_id)
        print(f"Submitting video model {model_id}", flush=True)
        request_name = f"video_{slugify(model_id)}_coverage.request.json"
        create_name = f"video_{slugify(model_id)}_coverage.create.json"
        status_name = f"video_{slugify(model_id)}_coverage.status.json"
        save_json(REQUESTS_DIR / request_name, payload)
        try:
            created_response = json_api("POST", f"{BASE_URL}/v1/videos", payload, timeout=120)
            save_json(RESPONSES_DIR / create_name, created_response)
            video_id = created_response.get("id")
            if not isinstance(video_id, str) or not video_id:
                raise RuntimeError(f"Unexpected create response: {json.dumps(created_response, ensure_ascii=False)}")
            created.append(
                {
                    "model_id": model_id,
                    "video_id": video_id,
                    "request_file": request_name,
                    "create_file": create_name,
                    "status_file": status_name,
                }
            )
        except Exception as exc:  # noqa: BLE001
            failures.append({"model_id": model_id, "error": str(exc)})
            print(f"Video submit failed for {model_id}: {exc}", file=sys.stderr, flush=True)
    try:
        finished = poll_videos(created)
        for item in finished:
            data, content_type = binary_request("GET", f"{BASE_URL}/v1/videos/{item['video_id']}/content")
            ext = infer_ext_from_content_type(content_type)
            if ext == ".bin":
                ext = ".mp4"
            media_name = file_name("video", item["model_id"], "coverage", 1, ext)
            media_path = MEDIA_DIR / media_name
            media_path.write_bytes(data)
            successes.append(
                {
                    "model_id": item["model_id"],
                    "strategy": "videos",
                    "video_id": item["video_id"],
                    "request": str((REQUESTS_DIR / item["request_file"]).relative_to(ROOT)),
                    "create_response": str((RESPONSES_DIR / item["create_file"]).relative_to(ROOT)),
                    "status_response": str((RESPONSES_DIR / item["status_file"]).relative_to(ROOT)),
                    "output": str(media_path.relative_to(ROOT)),
                }
            )
            print(f"Generated video via {item['model_id']}: {media_path.relative_to(ROOT)}", flush=True)
    except Exception as exc:  # noqa: BLE001
        failures.append({"stage": "poll_or_download", "error": str(exc)})
        print(f"Video polling failed: {exc}", file=sys.stderr, flush=True)
    manifest["videos"] = {"successes": successes, "failures": failures}


def tts_payload(model_id: str) -> dict[str, Any]:
    if model_id.startswith("gemini-"):
        return {
            "model": model_id,
            "voice": "Kore",
            "input": "A calm museum guide welcomes visitors into a skylit gallery and invites them to notice how the room changes in tone as they move forward.",
            "response_format": "wav",
            "instructions": "Speak clearly and warmly, with measured pacing and a polished public presentation tone.",
        }
    if model_id == "gpt-4o-mini-tts":
        return {
            "model": model_id,
            "voice": "verse",
            "input": "Gate A12 now begins boarding for the evening flight to Singapore. Business class and families with small children may board at this time.",
            "response_format": "mp3",
            "instructions": "Speak like a calm premium-airline announcement with clear diction and even pacing.",
        }
    return {
        "model": model_id,
        "voice": "alloy",
        "input": "Welcome to the gallery. Please move slowly through the next room and notice how the lighting changes the mood of the space.",
        "response_format": "mp3",
    }


def run_tts_chat_audio(model_id: str) -> dict[str, Any]:
    """Generate audio via /v1/chat/completions for gpt-4o-audio-preview (Fix 7)."""
    slug = "coverage"
    payload = {
        "model": model_id,
        "modalities": ["text", "audio"],
        "audio": {"voice": "alloy", "format": "wav"},
        "messages": [
            {
                "role": "user",
                "content": "Welcome to the gallery. Please move slowly through the next room and notice how the lighting changes the mood of the space.",
            }
        ],
    }
    request_name = f"audio_chat-{slugify(model_id)}_{slug}.request.json"
    response_name = f"audio_chat-{slugify(model_id)}_{slug}.response.json"
    save_json(REQUESTS_DIR / request_name, payload)
    response = json_api("POST", f"{BASE_URL}/v1/chat/completions", payload, timeout=120)
    save_json(RESPONSES_DIR / response_name, response)
    # Extract base64 audio from choices[].message.audio.data
    audio_b64 = None
    for choice in response.get("choices", []):
        audio_obj = choice.get("message", {}).get("audio", {})
        if isinstance(audio_obj, dict) and audio_obj.get("data"):
            audio_b64 = audio_obj["data"]
            break
    if not audio_b64:
        raise RuntimeError(f"No audio data in chat response for {model_id}")
    import base64 as b64mod
    data = b64mod.b64decode(audio_b64)
    ext = infer_ext_from_bytes(data)
    if ext in {".bin", ".json"}:
        ext = ".wav"
    media_name = file_name("audio", model_id, slug, 1, ext)
    media_path = MEDIA_DIR / media_name
    media_path.write_bytes(data)
    return {
        "model_id": model_id,
        "strategy": "chat_completions_audio",
        "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
        "response": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
        "output": str(media_path.relative_to(ROOT)),
    }


def run_tts(manifest: dict[str, Any]) -> None:
    models = filter_model_ids("AIHUBMIX_TTS_IDS", list_models("tts"))
    successes = []
    failures = []
    for model in models:
        model_id = model["model_id"]
        print(f"Starting TTS model {model_id}", flush=True)
        try:
            # --- Fix 7: gpt-4o-audio-preview via chat completions ---
            if model_id == "gpt-4o-audio-preview":
                entry = run_tts_chat_audio(model_id)
                successes.append(entry)
                print(f"Generated audio via {model_id} (chat): {entry.get('output', '')}", flush=True)
                continue
            payload = tts_payload(model_id)
            request_name = f"audio_{slugify(model_id)}_coverage.request.json"
            response_name = f"audio_{slugify(model_id)}_coverage.response-meta.json"
            save_json(REQUESTS_DIR / request_name, payload)
            data, content_type = binary_request("POST", f"{BASE_URL}/v1/audio/speech", payload)
            ext = infer_ext_from_content_type(content_type)
            if ext in {".bin", ".json"}:
                ext = infer_ext_from_bytes(data)
            media_name = file_name("audio", model_id, "coverage", 1, ext)
            media_path = MEDIA_DIR / media_name
            media_path.write_bytes(data)
            save_json(
                RESPONSES_DIR / response_name,
                {"model_id": model_id, "content_type": content_type, "detected_extension": ext, "bytes": len(data)},
            )
            successes.append(
                {
                    "model_id": model_id,
                    "strategy": "audio_speech",
                    "request": str((REQUESTS_DIR / request_name).relative_to(ROOT)),
                    "response_meta": str((RESPONSES_DIR / response_name).relative_to(ROOT)),
                    "output": str(media_path.relative_to(ROOT)),
                }
            )
            print(f"Generated audio via {model_id}: {media_path.relative_to(ROOT)}", flush=True)
        except Exception as exc:  # noqa: BLE001
            failures.append({"model_id": model_id, "error": str(exc)})
            print(f"TTS failed for {model_id}: {exc}", file=sys.stderr, flush=True)
    manifest["tts"] = {"successes": successes, "failures": failures}


def run_images(manifest: dict[str, Any]) -> None:
    models = filter_model_ids("AIHUBMIX_IMAGE_IDS", list_models("image_generation"))
    successes = []
    failures = []
    for model in models:
        model_id = model["model_id"]
        try:
            print(f"Starting image model {model_id}", flush=True)
            entry = run_image_model(model)
            successes.append(entry)
            outputs = entry.get("outputs", [])
            output_text = ", ".join(outputs) if outputs else "no media output"
            print(f"Processed image model {model_id}: {output_text}", flush=True)
        except Exception as exc:  # noqa: BLE001
            failures.append({"model_id": model_id, "error": str(exc)})
            print(f"Image failed for {model_id}: {exc}", file=sys.stderr, flush=True)
    manifest["images"] = {"successes": successes, "failures": failures}


def main() -> int:
    if not API_KEY:
        print("Set AIHUBMIX_API_KEY before running this script.", file=sys.stderr)
        return 1
    ensure_dirs()
    manifest: dict[str, Any] = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "base_url": BASE_URL,
        "seed_image": str(SEED_IMAGE_PATH.relative_to(ROOT)),
        "seed_video": str(SEED_VIDEO_PATH.relative_to(ROOT)),
        "seed_video_url": SEED_VIDEO_URL,
        "notes": "Files are saved exactly as returned by AIHubMix. No transcoding or metadata stripping is performed.",
    }
    stages = selected_stage_names()
    if "images" in stages:
        run_images(manifest)
    if "videos" in stages:
        run_videos(manifest)
    if "tts" in stages:
        run_tts(manifest)
    save_json(MANIFEST_PATH, manifest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
