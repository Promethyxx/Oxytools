"""
Oxycash – Storage layer
  • WebDAV (Nextcloud / kDrive) — primary
  • Local JSON file (~/.oxycash/oxycash.json) — fallback
"""
from __future__ import annotations
import json, os, pathlib, base64
from typing import Optional, Tuple
import urllib.request, urllib.error

from .model import AppData, default_data

LOCAL_DIR  = pathlib.Path.home() / '.oxycash'
LOCAL_FILE = LOCAL_DIR / 'oxycash.json'
CONF_FILE  = LOCAL_DIR / 'config.json'
JSON_NAME  = 'oxycash.json'


# ─── Config ───────────────────────────────────────────────────────────────────

def load_config() -> dict:
    if CONF_FILE.exists():
        try:
            return json.loads(CONF_FILE.read_text(encoding='utf-8'))
        except Exception:
            pass
    return {'dav_url': '', 'dav_user': '', 'dav_pass': '', 'lang': 'en'}


def save_config(cfg: dict):
    LOCAL_DIR.mkdir(parents=True, exist_ok=True)
    CONF_FILE.write_text(json.dumps(cfg, indent=2), encoding='utf-8')


# ─── WebDAV helpers ───────────────────────────────────────────────────────────

def _auth_header(user: str, pw: str) -> str:
    return 'Basic ' + base64.b64encode(f"{user}:{pw}".encode()).decode()


def _dav_url(cfg: dict) -> Optional[str]:
    url  = cfg.get('dav_url', '').strip()
    user = cfg.get('dav_user', '').strip()
    pw   = cfg.get('dav_pass', '').strip()
    if not (url and user and pw):
        return None
    return (url if url.endswith('/') else url + '/') + JSON_NAME


UA = 'Oxycash/1.0 (Python urllib)'


def _dav_req(url, method, user, pw, body=None):
    req = urllib.request.Request(url, data=body, method=method)
    req.add_header('Authorization', _auth_header(user, pw))
    req.add_header('User-Agent', UA)
    req.add_header('OCS-APIREQUEST', 'true')
    if body:
        req.add_header('Content-Type', 'application/json; charset=utf-8')
    return req


def dav_test(cfg: dict) -> Tuple[bool, str]:
    """Returns (ok, message)"""
    url  = cfg.get('dav_url', '').strip()
    user = cfg.get('dav_user', '').strip()
    pw   = cfg.get('dav_pass', '').strip()
    if not (url and user and pw):
        return False, "Remplis les 3 champs"
    full = (url if url.endswith('/') else url + '/') + JSON_NAME
    req = _dav_req(full, 'HEAD', user, pw)
    try:
        with urllib.request.urlopen(req, timeout=8) as r:
            return True, f"OK ({r.status}) — {full}"
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return True, f"OK — dossier accessible, fichier sera cree"
        if e.code == 401:
            return False, "401 Non autorise — verifie user/mot de passe"
        if e.code == 403:
            return False, f"403 Interdit — verifie que le dossier existe sur Nextcloud: {full}"
        return False, f"Erreur HTTP {e.code} — {full}"
    except Exception as ex:
        return False, str(ex)


def dav_load(cfg: dict) -> Optional[AppData]:
    full = _dav_url(cfg)
    if not full:
        return None
    user = cfg['dav_user'].strip()
    pw   = cfg['dav_pass'].strip()
    req  = _dav_req(full, 'GET', user, pw)
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            raw = r.read().decode('utf-8')
            d = json.loads(raw)
            if 'months' in d:
                return AppData.from_dict(d)
    except urllib.error.HTTPError as e:
        if e.code != 404:
            raise
    except Exception:
        pass
    return None


def dav_save(cfg: dict, data: AppData) -> bool:
    full = _dav_url(cfg)
    if not full:
        return False
    user = cfg['dav_user'].strip()
    pw   = cfg['dav_pass'].strip()
    body = data.to_json().encode('utf-8')
    req  = _dav_req(full, 'PUT', user, pw, body)
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status in (200, 201, 204)
    except Exception:
        return False


# ─── Local JSON ───────────────────────────────────────────────────────────────

def local_load() -> Optional[AppData]:
    if LOCAL_FILE.exists():
        try:
            d = json.loads(LOCAL_FILE.read_text(encoding='utf-8'))
            if 'months' in d:
                return AppData.from_dict(d)
        except Exception:
            pass
    return None


def local_save(data: AppData):
    LOCAL_DIR.mkdir(parents=True, exist_ok=True)
    LOCAL_FILE.write_text(data.to_json(), encoding='utf-8')


# ─── High-level storage manager ───────────────────────────────────────────────

class Storage:
    """
    Manages persistence with WebDAV primary + local fallback.
    All methods are synchronous (call from a thread if needed).
    """
    def __init__(self):
        self.cfg    = load_config()
        self.dav_ok = False
        self.data   = AppData()

    def reload_config(self):
        self.cfg = load_config()

    def save_config(self, url: str, user: str, pw: str):
        lang = self.cfg.get('lang', 'en')
        self.cfg = {'dav_url': url, 'dav_user': user, 'dav_pass': pw, 'lang': lang}
        save_config(self.cfg)

    def clear_config(self):
        lang = self.cfg.get('lang', 'en')
        self.cfg = {'dav_url': '', 'dav_user': '', 'dav_pass': '', 'lang': lang}
        save_config(self.cfg)
        self.dav_ok = False

    def set_lang(self, lang: str):
        from .model import MONTHS  # avoid circular
        from .i18n import set_lang
        self.cfg['lang'] = lang
        save_config(self.cfg)
        set_lang(lang)

    def load_lang(self):
        from .i18n import set_lang
        set_lang(self.cfg.get('lang', 'en'))

    def load(self) -> str:
        """Load data; return 'dav' | 'local' | 'default'"""
        from .model import apply_recurring
        # try WebDAV
        try:
            app = dav_load(self.cfg)
            if app:
                self.data   = app
                self.dav_ok = True
                apply_recurring(self.data)
                local_save(app)
                return 'dav'
        except Exception:
            pass
        self.dav_ok = False
        # try local
        app = local_load()
        if app:
            self.data = app
            apply_recurring(self.data)
            return 'local'
        # default
        self.data = default_data()
        apply_recurring(self.data)
        local_save(self.data)
        return 'default'

    def save(self) -> str:
        """Save data; return 'dav' | 'local'"""
        local_save(self.data)
        if _dav_url(self.cfg):
            ok = dav_save(self.cfg, self.data)
            self.dav_ok = ok
            if ok:
                return 'dav'
        self.dav_ok = False
        return 'local'

    def test_dav(self) -> Tuple[bool, str]:
        return dav_test(self.cfg)

    def status(self) -> str:
        """'dav' | 'dav_err' | 'local'"""
        if self.dav_ok:
            return 'dav'
        if _dav_url(self.cfg):
            return 'dav_err'
        return 'local'

    # ── Import / Export ──
    def export_json(self) -> str:
        return self.data.to_json()

    def import_json(self, raw: str) -> bool:
        try:
            app = AppData.from_json(raw)
            if app.months:
                self.data = app
                self.save()
                return True
        except Exception:
            pass
        return False

    def reset(self):
        self.data = default_data()
        self.save()
