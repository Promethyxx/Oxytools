"""
Oxycash 2026 — Flet 0.82+
Run:   python main.py
Build: flet build windows / linux / apk
"""
from __future__ import annotations
import flet as ft
import threading

from core.model   import MONTHS, MNAMES, ALL_TABS, detect_budget_month
from core.storage import Storage
from core.theme   import Theme
from core.i18n    import T, set_lang
from views        import (
    build_month_view, build_dettes_view, build_epargne_view,
    build_frais_view, build_viabilite_view, build_config_view,
)

def get_tab_labels():
    return T.months_short + [
        T['tab_debts'], T['tab_savings'], T['tab_expenses'],
        T['tab_viability'], T['tab_config'],
    ]

P = ft.Padding
B = ft.Border
BS = ft.BorderSide


def main(page: ft.Page):
    page.title   = 'Oxycash'
    page.bgcolor = '#0D0D0D'
    page.padding = 0
    page.fonts   = {
        'Playfair Display': 'https://fonts.gstatic.com/s/playfairdisplay/v37/nuFiD-vYSZviVYUb_rj3ij__anPXDTzYgEM86xRbAA.ttf',
        'DM Sans': 'https://fonts.gstatic.com/s/dmsans/v14/rP2Hp2ywxg089UriCZa4ET-DNl0.ttf',
    }

    # ── state ──
    storage  = Storage()
    theme    = Theme(dark=True)
    state    = {'tab': detect_budget_month()}
    _toast_timer = [None]

    def c(k): return theme.c(k)

    # ── toast ──────────────────────────────────────────────────────────────
    toast_txt = ft.Text('', size=13, color='#1a1a1a', font_family='DM Sans',
                         weight=ft.FontWeight.W_500)
    toast_box = ft.Container(
        toast_txt,
        padding=P.symmetric(horizontal=20, vertical=10),
        bgcolor='#7BC47F', border_radius=10,
        visible=False,
        margin=P.symmetric(horizontal=20),
    )

    def show_toast(msg: str):
        toast_txt.value  = msg
        toast_box.visible = True
        try: toast_txt.update(); toast_box.update()
        except: pass
        if _toast_timer[0]: _toast_timer[0].cancel()
        def hide():
            toast_box.visible = False
            try: toast_box.update()
            except: pass
        _toast_timer[0] = threading.Timer(2.2, hide)
        _toast_timer[0].start()

    # ── badge ───────────────────────────────────────────────────────────────
    badge_txt = ft.Text('💾 Local', size=10, font_family='DM Sans',
                         weight=ft.FontWeight.W_600, color='#F2D388')
    badge = ft.Container(badge_txt,
                         padding=P.symmetric(horizontal=8, vertical=3),
                         border_radius=8, bgcolor='rgba(242,211,136,0.15)')

    def update_badge():
        s = storage.status()
        if s == 'dav':
            badge_txt.value  = '☁️ WebDAV';     badge_txt.color = '#7BC47F'
            badge.bgcolor    = 'rgba(123,196,127,0.15)'
        elif s == 'dav_err':
            badge_txt.value  = '⚠️ Déconnecté'; badge_txt.color = '#E05555'
            badge.bgcolor    = 'rgba(224,85,85,0.15)'
        else:
            badge_txt.value  = '💾 Local';      badge_txt.color = '#F2D388'
            badge.bgcolor    = 'rgba(242,211,136,0.15)'
        try: badge.update()
        except: pass

    # ── content area ────────────────────────────────────────────────────────
    content_col = ft.Column(expand=True, scroll=ft.ScrollMode.AUTO)

    def on_save():
        def _do():
            storage.save()
            update_badge()
        threading.Thread(target=_do, daemon=True).start()

    def render():
        theme.scale = font_scale[0]
        tab = state['tab']

        # rebuild tabs
        tabs = []
        for i, key in enumerate(ALL_TABS):
            is_act  = key == tab
            is_spec = i >= 12
            act_bg  = c('teal') if is_spec else c('gold')
            norm_fg = c('teal') if is_spec else c('text3')
            def make_tap(k):
                def _tap(e):
                    state['tab'] = k
                    render()
                return _tap
            tabs.append(ft.GestureDetector(
                content=ft.Container(
                    ft.Text(get_tab_labels()[i], size=11, weight=ft.FontWeight.W_600,
                            font_family='DM Sans', no_wrap=True,
                            color='#1a1a1a' if is_act else norm_fg),
                    padding=P.symmetric(horizontal=14, vertical=6),
                    bgcolor=act_bg if is_act else 'transparent',
                    border_radius=8,
                ),
                on_tap=make_tap(key),
            ))

        tab_row.controls.clear()
        tab_row.controls.extend(tabs)
        try: tab_row.update()
        except: pass

        # rebuild content
        mi = MONTHS.index(tab) if tab in MONTHS else -1
        if mi >= 0:
            view = build_month_view(tab, storage.data.months[tab], theme, on_save, show_toast, all_months=storage.data.months, page=page, frais=storage.data.frais)
        elif tab == 'Debts':
            view = build_dettes_view(storage.data, theme, on_save, show_toast)
        elif tab == 'Savings':
            view = build_epargne_view(storage.data, theme, on_save, show_toast)
        elif tab == 'Expenses':
            view = build_frais_view(storage.data, theme, on_save, show_toast)
        elif tab == 'Viability':
            view = build_viabilite_view(storage.data, theme, on_save, show_toast)
        elif tab == 'Config':
            view = build_config_view(storage, theme, on_save, show_toast, render,
                                     lambda: (theme.toggle(), _apply_theme(), render()),
                                     page=page)
        else:
            view = ft.Text('?')

        content_col.controls.clear()
        content_col.controls.append(
            ft.Container(view, padding=P.symmetric(horizontal=14, vertical=12), expand=True)
        )
        try: content_col.update()
        except: pass

    def _apply_theme():
        page.bgcolor = c('bg')
        top_bar.bgcolor = c('bg2')
        try: page.update()
        except: pass

    # ── font scale ──────────────────────────────────────────────────────────
    font_scale = [0]   # offset in px, persisted in config

    def _scale_btn(label, delta):
        def _tap(e):
            font_scale[0] = max(-6, min(8, font_scale[0] + delta))
            storage.cfg['font_scale'] = font_scale[0]
            from core.storage import save_config
            save_config(storage.cfg)
            render()
        return ft.GestureDetector(
            content=ft.Container(
                ft.Text(label, size=11, font_family='DM Sans',
                        weight=ft.FontWeight.W_700, color=c('text2')),
                width=28, height=28,
                border=B.all(1, c('card_border')), border_radius=6,
                alignment=ft.Alignment(0, 0),
            ),
            on_tap=_tap,
        )

    font_scale[0] = storage.cfg.get('font_scale', 0)

    # ── lang toggle ─────────────────────────────────────────────────────────
    def _lang_display():
        from core.i18n import get_lang
        return 'FR' if get_lang() == 'en' else 'EN'

    def do_toggle_lang(e):
        from core.i18n import toggle_lang, get_lang
        toggle_lang()
        storage.set_lang(get_lang())
        render()

    # ── theme toggle ────────────────────────────────────────────────────────
    theme_icon = ft.Text('🌙', size=16)
    def toggle_theme(e):
        theme.toggle()
        theme_icon.value = '☀️' if not theme.is_dark else '🌙'
        _apply_theme()
        render()
        try: theme_icon.update()
        except: pass

    # ── top bar ─────────────────────────────────────────────────────────────
    tab_row = ft.Row([], spacing=2, scroll=ft.ScrollMode.AUTO)

    top_bar = ft.Container(
        ft.Column([
            ft.Row([
                ft.Row([
                    ft.Text('Oxycash', size=18, weight=ft.FontWeight.W_700,
                            font_family='Playfair Display', color=c('text')),
                    badge,
                ], spacing=6, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                ft.Row([
                    _scale_btn('A-', -2),
                    _scale_btn('A+', +2),
                    ft.GestureDetector(
                        content=ft.Container(
                            ft.Text(_lang_display(), size=10, font_family='DM Sans',
                                    weight=ft.FontWeight.W_700, color=c('gold')),
                            padding=P.symmetric(horizontal=8, vertical=7),
                            border=B.all(1, c('gold')), border_radius=8,
                        ),
                        on_tap=do_toggle_lang,
                    ),
                    ft.Container(
                        theme_icon, width=32, height=32,
                        border_radius=16,
                        border=B.all(1, c('card_border')),
                        alignment=ft.Alignment(0, 0),
                        on_click=toggle_theme, ink=True,
                    ),
                ], spacing=6),
            ], alignment=ft.MainAxisAlignment.SPACE_BETWEEN,
               vertical_alignment=ft.CrossAxisAlignment.CENTER),
            ft.Container(tab_row, padding=P.only(bottom=10, top=4)),
        ], spacing=0),
        bgcolor=c('bg2'),
        padding=P.only(left=12, right=12, top=10, bottom=0),
        border=B.only(bottom=BS(1, c('card_border'))),
    )

    # ── root ────────────────────────────────────────────────────────────────
    page.add(ft.Column([
        top_bar,
        ft.Container(content_col, expand=True),
        ft.Container(toast_box, alignment=ft.Alignment(0, 0)),
    ], expand=True, spacing=0))

    # ── initial load ────────────────────────────────────────────────────────
    def do_load():
        storage.load()
        storage.load_lang()
        update_badge()
        render()
    threading.Thread(target=do_load, daemon=True).start()


if __name__ == '__main__':
    ft.app(target=main)