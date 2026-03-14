"""Oxycash – Special views (Flet 0.82+) — fully editable"""
from __future__ import annotations
import flet as ft
from core.model import AppData, fmt, MONTHS

P  = ft.Padding
M  = ft.Margin
BR = ft.BorderRadius
B  = ft.Border
BS = ft.BorderSide


def _t(s, size=12, weight=ft.FontWeight.NORMAL, col='', family='DM Sans',
       align=ft.TextAlign.LEFT, expand=False, overflow=None, no_wrap=False):
    kw = dict(size=size, weight=weight, font_family=family, text_align=align)
    if col:      kw['color']    = col
    if expand:   kw['expand']   = True
    if overflow: kw['overflow'] = overflow
    if no_wrap:  kw['no_wrap']  = True
    return ft.Text(str(s), **kw)


def _tf(value, on_change=None, on_blur=None, width=None, expand=False,
        num=False, col='', hint='', password=False, c=None):
    kw = dict(
        value=str(value), text_size=12, color=col,
        bgcolor='transparent', border_color=c('card_border'),
        focused_border_color=c('gold'),
        content_padding=P.symmetric(horizontal=8, vertical=6),
        hint_text=hint, password=password, can_reveal_password=password,
    )
    if width:  kw['width']  = width
    if expand: kw['expand'] = True
    if num:    kw['keyboard_type'] = ft.KeyboardType.NUMBER
    if on_change: kw['on_change'] = on_change
    if on_blur:   kw['on_blur']   = on_blur
    return ft.TextField(**kw)


def _sec_hdr(icon, label, total_str, col_key, c):
    return ft.Container(
        ft.Row([_t(icon, size=16, col=c('text')),
                _t(label, size=14, weight=ft.FontWeight.W_600,
                   family='Playfair Display', col=c('text'), expand=True),
                _t(total_str, size=13, weight=ft.FontWeight.W_700, col=c(col_key))],
               spacing=8),
        padding=P.only(bottom=6, top=4),
        border=B.only(bottom=BS(1, c('card_border'))),
    )


def _add_btn(label, on_click, c):
    return ft.GestureDetector(
        content=ft.Container(
            ft.Row([ft.Icon(ft.Icons.ADD, size=14, color=c('text3')),
                    _t(label, size=12, col='text3')], spacing=6),
            padding=P.symmetric(horizontal=12, vertical=8),
            border=B.all(1, c('card_border')), border_radius=10,
        ),
        on_tap=on_click,
    )


def _del_btn(on_click, c):
    return ft.IconButton(
        ft.Icons.DELETE_OUTLINE, icon_size=16, icon_color=c('danger'),
        on_click=on_click, style=ft.ButtonStyle(padding=P.all(2)),
    )


# ═══ DETTES ════════════════════════════════════════════════════════

def build_dettes_view(data: AppData, t, on_save, on_toast):
    def c(k): return t.c(k)
    col = ft.Column([], spacing=8, scroll=ft.ScrollMode.AUTO, expand=True)

    def rebuild():
        col.controls.clear()
        col.controls.extend(_build())
        try: col.update()
        except: pass

    def _build():
        total_du = sum(d.solde   for d in data.dettes)
        total_ok = sum(d.soldeOk for d in data.dettes)
        paid     = sum(1 for d in data.dettes if d.soldeOk >= d.solde)

        summary = ft.Container(
            ft.Row([
                ft.Column([_t('TOTAL DU', size=9, weight=ft.FontWeight.W_600, col=c('text3')),
                           _t(f"{fmt(total_du)} CHF", size=16, weight=ft.FontWeight.W_700, col=c('danger'))],
                          expand=True),
                ft.Column([_t('NEGOCIE', size=9, weight=ft.FontWeight.W_600, col=c('text3')),
                           _t(f"{fmt(total_ok)} CHF", size=16, weight=ft.FontWeight.W_700, col=c('green'))],
                          expand=True),
                ft.Column([_t('SOLDEES', size=9, weight=ft.FontWeight.W_600, col=c('text3')),
                           _t(f"{paid}/{len(data.dettes)}", size=16, weight=ft.FontWeight.W_700, col=c('teal'))],
                          expand=True),
            ]),
            padding=14, bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=10,
        )

        cards = []
        for i, d in enumerate(data.dettes):
            is_paid = d.soldeOk >= d.solde
            accent  = c('green') if is_paid else c('danger')

            def upd_str(field, i=i):
                def _h(e): setattr(data.dettes[i], field, e.control.value); on_save()
                return _h
            def upd_num(field, i=i):
                def _h(e):
                    try: setattr(data.dettes[i], field, float(e.control.value.replace(',','.'))); on_save(); rebuild()
                    except ValueError: pass
                return _h
            def del_dette(e, i=i):
                data.dettes.pop(i); on_save(); rebuild(); on_toast('Supprime')

            cards.append(ft.Container(
                ft.Column([
                    ft.Row([
                        _tf(d.creancier, on_blur=upd_str('creancier'), expand=True, col=c('text'), c=c),
                        _del_btn(del_dette, c),
                    ], spacing=4),
                    ft.Row([
                        ft.Column([_t('Representant', size=9, col=c('text3')),
                                   _tf(d.rep, on_blur=upd_str('rep'), col=c('text2'), c=c)],
                                  expand=True, spacing=2),
                        ft.Column([_t('N Poursuite', size=9, col=c('text3')),
                                   _tf(d.poursuite, on_blur=upd_str('poursuite'), col=c('text2'), c=c)],
                                  expand=True, spacing=2),
                    ], spacing=8),
                    ft.Row([
                        ft.Column([_t('Solde du (CHF)', size=9, col=c('text3')),
                                   _tf(fmt(d.solde), on_blur=upd_num('solde'), num=True, col=c('danger'), c=c)],
                                  expand=True, spacing=2),
                        ft.Column([_t('Negocie (CHF)', size=9, col=c('text3')),
                                   _tf(fmt(d.soldeOk), on_blur=upd_num('soldeOk'), num=True, col=c('green'), c=c)],
                                  expand=True, spacing=2),
                    ], spacing=8),
                    ft.Row([
                        ft.Column([_t('Etat', size=9, col=c('text3')),
                                   _tf(d.etat, on_blur=upd_str('etat'), col=c('text2'), c=c)],
                                  expand=True, spacing=2),
                        ft.Column([_t('Date', size=9, col=c('text3')),
                                   _tf(d.date, on_blur=upd_str('date'), col=c('text3'), c=c)],
                                  expand=True, spacing=2),
                    ], spacing=8),
                ], spacing=6),
                padding=14, bgcolor=c('card'),
                border=B.only(left=BS(3, accent), top=BS(1, c('card_border')),
                              right=BS(1, c('card_border')), bottom=BS(1, c('card_border'))),
                border_radius=10, opacity=0.75 if is_paid else 1.0,
            ))

        def add_dette(e):
            from core.model import Dette
            data.dettes.append(Dette(rep='', creancier='Nouveau', poursuite='',
                                     solde=0, soldeOk=0, etat='', date=''))
            on_save(); rebuild()

        return [
            _t('Dettes', size=20, weight=ft.FontWeight.W_700, family='Playfair Display', col=c('text')),
            _t('Suivi des poursuites et creances', size=12, col=c('text2')),
            summary, *cards,
            _add_btn('Ajouter une dette', add_dette, c),
            ft.Container(height=40),
        ]

    col.controls.extend(_build())
    return col


# ═══ EPARGNE ════════════════════════════════════════════════════════

def build_epargne_view(data: AppData, t, on_save, on_toast):
    def c(k): return t.c(k)
    col = ft.Column([], spacing=6, scroll=ft.ScrollMode.AUTO, expand=True)

    def rebuild():
        col.controls.clear()
        col.controls.extend(_build())
        try: col.update()
        except: pass

    def _build():
        ep = data.epargne
        sond_total   = sum(s.total for s in ep.get('sondages', []))
        achats_total = sum(a.prix  for a in ep.get('achats',   []))
        legacy_total = sum(p.val   for p in ep.get('pc_legacy',[]))

        sond_rows = []
        for i, s in enumerate(ep.get('sondages', [])):
            pct = min(1.0, s.total / s.goal) if s.goal > 0 else 0
            def upd_sn(i=i):
                def _h(e): ep['sondages'][i].name = e.control.value; on_save()
                return _h
            def upd_sf(field, i=i):
                def _h(e):
                    try: setattr(ep['sondages'][i], field, float(e.control.value.replace(',','.'))); on_save(); rebuild()
                    except ValueError: pass
                return _h
            def del_s(e, i=i):
                ep['sondages'].pop(i); on_save(); rebuild()

            sond_rows.append(ft.Container(
                ft.Column([
                    ft.Row([_tf(s.name, on_blur=upd_sn(), expand=True, col=c('text'), c=c),
                            _del_btn(del_s, c)], spacing=4),
                    ft.Row([
                        ft.Column([_t('Total CHF', size=9, col=c('text3')),
                                   _tf(fmt(s.total), on_blur=upd_sf('total'), num=True, col=c('teal'), c=c)],
                                  expand=True, spacing=2),
                        ft.Column([_t('Objectif CHF', size=9, col=c('text3')),
                                   _tf(fmt(s.goal), on_blur=upd_sf('goal'), num=True, col=c('text2'), c=c)],
                                  expand=True, spacing=2),
                    ], spacing=8),
                    ft.Container(ft.Container(width=pct, bgcolor=c('teal'), border_radius=3),
                                 height=4, bgcolor=c('card_border'), border_radius=3)
                    if s.goal > 0 else ft.Container(height=4),
                ], spacing=4),
                padding=P.symmetric(horizontal=12, vertical=10),
                bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=8,
            ))

        def add_sond(e):
            from core.model import EpargneSondage
            ep['sondages'].append(EpargneSondage('Nouveau', 0, 0)); on_save(); rebuild()

        achat_rows = []
        for i, a in enumerate(ep.get('achats', [])):
            def upd_an(i=i):
                def _h(e): ep['achats'][i].name = e.control.value; on_save()
                return _h
            def upd_ap(i=i):
                def _h(e):
                    try: ep['achats'][i].prix = float(e.control.value.replace(',','.')); on_save(); rebuild()
                    except ValueError: pass
                return _h
            def del_a(e, i=i):
                ep['achats'].pop(i); on_save(); rebuild()

            achat_rows.append(ft.Container(
                ft.Row([_tf(a.name, on_blur=upd_an(), expand=True, col=c('text'), c=c),
                        _tf(fmt(a.prix), on_blur=upd_ap(), num=True, width=80, col=c('gold'), c=c),
                        _del_btn(del_a, c)],
                       spacing=4, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                padding=P.symmetric(horizontal=12, vertical=8),
                bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=8,
            ))

        def add_achat(e):
            from core.model import EpargneAchat
            ep['achats'].append(EpargneAchat('Nouveau', 0)); on_save(); rebuild()

        legacy_rows = []
        for i, p in enumerate(ep.get('pc_legacy', [])):
            def upd_pn(i=i):
                def _h(e): ep['pc_legacy'][i].name = e.control.value; on_save()
                return _h
            def upd_pv(i=i):
                def _h(e):
                    try: ep['pc_legacy'][i].val = float(e.control.value.replace(',','.')); on_save(); rebuild()
                    except ValueError: pass
                return _h
            def del_p(e, i=i):
                ep['pc_legacy'].pop(i); on_save(); rebuild()

            legacy_rows.append(ft.Container(
                ft.Row([_tf(p.name, on_blur=upd_pn(), expand=True, col=c('text'), c=c),
                        _tf(fmt(p.val), on_blur=upd_pv(), num=True, width=80, col=c('purple'), c=c),
                        _del_btn(del_p, c)],
                       spacing=4, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                padding=P.symmetric(horizontal=12, vertical=8),
                bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=8,
            ))

        def add_legacy(e):
            from core.model import EpargnePcLegacy
            ep['pc_legacy'].append(EpargnePcLegacy('Nouveau', 0)); on_save(); rebuild()

        sc = 'green' if legacy_total >= achats_total else 'danger'
        return [
            _t('Epargne', size=20, weight=ft.FontWeight.W_700, family='Playfair Display', col=c('text')),
            _sec_hdr('Sondages en cours', '', f"{fmt(sond_total)} CHF", 'teal', c),
            *sond_rows, _add_btn('Ajouter un sondage', add_sond, c),
            ft.Container(height=8),
            _sec_hdr('Wishlist PC', '', f"{fmt(achats_total)} CHF", 'gold', c),
            *achat_rows, _add_btn('Ajouter un achat', add_achat, c),
            ft.Container(height=8),
            _sec_hdr('PC Legacy (revente)', '', f"{fmt(legacy_total)} CHF", 'purple', c),
            *legacy_rows, _add_btn('Ajouter une piece', add_legacy, c),
            ft.Container(
                ft.Row([_t('Solde net', size=13, weight=ft.FontWeight.W_700, col=c('text'), expand=True),
                        _t(f"{fmt(legacy_total - achats_total)} CHF", size=16,
                           weight=ft.FontWeight.W_700, col=c(sc))]),
                padding=14, bgcolor=c('card'), border=B.all(1, c('card_border')),
                border_radius=10, margin=M.only(top=8),
            ),
            ft.Container(height=40),
        ]

    col.controls.extend(_build())
    return col


# ═══ FRAIS ══════════════════════════════════════════════════════════

def build_frais_view(data: AppData, t, on_save, on_toast):
    def c(k): return t.c(k)
    col = ft.Column([], spacing=12, scroll=ft.ScrollMode.AUTO, expand=True)

    def rebuild():
        col.controls.clear()
        col.controls.extend(_build())
        try: col.update()
        except: pass

    def _build():
        def table(cat, label, col_key):
            lines  = data.frais.get(cat, [])
            totals = [sum(l.monthly[mi] for l in lines) for mi in range(12)]
            grand  = sum(totals)

            def hdr(s, ck, width=None):
                ct = _t(s, size=9, weight=ft.FontWeight.W_600, col=c(ck), align=ft.TextAlign.CENTER)
                return ft.Container(ct, width=width) if width else ft.Container(ct, expand=True)

            header = ft.Row(
                [hdr('Poste', 'text3', width=72)] +
                [hdr(MONTHS[mi][:3], 'text3') for mi in range(12)] +
                [hdr('Total', col_key, width=52), ft.Container(width=28)],
                spacing=2,
            )
            rows = [header, ft.Divider(height=1, color=c('card_border'))]

            for li, fl in enumerate(lines):
                yr = sum(fl.monthly)

                def upd_name(e, li=li, cat=cat):
                    data.frais[cat][li].name = e.control.value; on_save()

                month_cells = []
                for mi in range(12):
                    def upd_m(e, li=li, mi=mi, cat=cat):
                        try:
                            data.frais[cat][li].monthly[mi] = float(e.control.value.replace(',','.') or '0')
                            on_save(); rebuild()
                        except ValueError: pass
                    month_cells.append(ft.Container(
                        ft.TextField(
                            value=fmt(fl.monthly[mi]) if fl.monthly[mi] else '',
                            hint_text='0', text_size=9,
                            text_align=ft.TextAlign.CENTER,
                            color=c('text'), bgcolor='transparent',
                            border_color=c('card_border'),
                            focused_border_color=c('gold'),
                            content_padding=P.symmetric(horizontal=2, vertical=4),
                            keyboard_type=ft.KeyboardType.NUMBER,
                            on_blur=upd_m,
                        ), expand=True,
                    ))

                def del_line(e, li=li, cat=cat):
                    data.frais[cat].pop(li); on_save(); rebuild()

                rows.append(ft.Row(
                    [ft.Container(
                        ft.TextField(value=fl.name, text_size=10, color=c('text2'),
                                     bgcolor='transparent', border_color='transparent',
                                     focused_border_color=c('gold'),
                                     content_padding=P.symmetric(horizontal=4, vertical=4),
                                     on_blur=upd_name),
                        width=72)] +
                    month_cells +
                    [ft.Container(_t(fmt(yr), size=10, weight=ft.FontWeight.W_700,
                                      col=c(col_key), align=ft.TextAlign.RIGHT), width=52),
                     _del_btn(del_line, c)],
                    spacing=2,
                ))

            rows.append(ft.Divider(height=1, color=c('card_border')))
            rows.append(ft.Row(
                [ft.Container(_t('Total', size=10, weight=ft.FontWeight.W_700, col=c('text')), width=72)] +
                [ft.Container(_t(fmt(totals[mi]) if totals[mi] else '-', size=9,
                                  weight=ft.FontWeight.W_700, col=c(col_key),
                                  align=ft.TextAlign.CENTER), expand=True)
                 for mi in range(12)] +
                [ft.Container(_t(fmt(grand), size=10, weight=ft.FontWeight.W_700,
                                  col=c(col_key), align=ft.TextAlign.RIGHT), width=52),
                 ft.Container(width=28)],
                spacing=2,
            ))

            def add_line(e, cat=cat):
                from core.model import FraisLine
                data.frais[cat].append(FraisLine('Nouveau', [0.0]*12)); on_save(); rebuild()

            return ft.Column([
                ft.Container(_t(label, size=13, weight=ft.FontWeight.W_600,
                                 family='Playfair Display', col=c('text')),
                             padding=P.only(bottom=6, top=4),
                             border=B.only(bottom=BS(1, c('card_border')))),
                ft.Container(ft.Column(rows, spacing=4), padding=12,
                             bgcolor=c('card'), border=B.all(1, c('card_border')),
                             border_radius=10, clip_behavior=ft.ClipBehavior.HARD_EDGE),
                _add_btn('Ajouter une ligne', add_line, c),
            ], spacing=6)

        return [
            _t('Frais annuels', size=20, weight=ft.FontWeight.W_700,
               family='Playfair Display', col=c('text')),
            table('fixes',     'Charges fixes',    'blue'),
            table('ponctuels', 'Frais ponctuels',  'amber'),
            ft.Container(height=40),
        ]

    col.controls.extend(_build())
    return col


# ═══ VIABILITE ══════════════════════════════════════════════════════

def build_viabilite_view(data: AppData, t, on_save, on_toast):
    def c(k): return t.c(k)
    col = ft.Column([], spacing=10, scroll=ft.ScrollMode.AUTO, expand=True)

    def rebuild():
        col.controls.clear()
        col.controls.extend(_build())
        try: col.update()
        except: pass

    def _build():
        COLS = [('Salaire','gold',60),('Loyer','text3',None),('Assur.','text3',None),
                ('Frais','text3',None),('Impot','text3',None),('Subside','teal',None),('Solde','green',58)]

        def hdr(s, ck, width=None):
            ct = _t(s, size=9, weight=ft.FontWeight.W_600, col=c(ck), align=ft.TextAlign.RIGHT)
            return ft.Container(ct, width=width) if width else ft.Container(ct, expand=True)

        header = ft.Row(
            [hdr(s, ck, w) for s, ck, w in COLS] + [ft.Container(width=28)],
            spacing=4,
        )
        rows = [header, ft.Divider(height=1, color=c('card_border'))]

        for vi, v in enumerate(data.viabilite):
            charges = v.loyer + v.assurance + v.fraisMin + v.impotM - v.subside
            solde   = v.salaire - charges
            sc      = 'green' if solde >= 0 else 'danger'

            def upd(field, vi=vi):
                def _h(e):
                    try:
                        setattr(data.viabilite[vi], field, float(e.control.value.replace(',','.') or '0'))
                        on_save(); rebuild()
                    except ValueError: pass
                return _h

            def del_v(e, vi=vi):
                data.viabilite.pop(vi); on_save(); rebuild()

            def cell(val, field, width=None, ck='text2'):
                tf = ft.TextField(
                    value=fmt(val), text_size=10, text_align=ft.TextAlign.RIGHT,
                    color=c(ck), bgcolor='transparent', border_color='transparent',
                    focused_border_color=c('gold'),
                    content_padding=P.symmetric(horizontal=4, vertical=4),
                    keyboard_type=ft.KeyboardType.NUMBER, on_blur=upd(field),
                )
                return ft.Container(tf, width=width) if width else ft.Container(tf, expand=True)

            rows.append(ft.Row([
                cell(v.salaire,   'salaire',   60,   'gold'),
                cell(v.loyer,     'loyer',     None, 'text2'),
                cell(v.assurance, 'assurance', None, 'text2'),
                cell(v.fraisMin,  'fraisMin',  None, 'text2'),
                cell(v.impotM,    'impotM',    None, 'text2'),
                cell(v.subside,   'subside',   None, 'teal'),
                ft.Container(_t(fmt(solde), size=11, weight=ft.FontWeight.W_700,
                                 col=c(sc), align=ft.TextAlign.RIGHT), width=58),
                _del_btn(del_v, c),
            ], spacing=4))

        def add_palier(e):
            from core.model import ViabilitePalier
            last = data.viabilite[-1] if data.viabilite else None
            s = (last.salaire + 500) if last else 3000
            ass = 520 if s <= 6000 else 495 if s <= 7000 else 444
            data.viabilite.append(ViabilitePalier(s, 1412, ass, 500, round(s*0.125), 0))
            on_save(); rebuild()

        return [
            _t('Viabilite', size=20, weight=ft.FontWeight.W_700,
               family='Playfair Display', col=c('text')),
            _t('Simulation mensuelle par palier de salaire', size=12, col=c('text2')),
            ft.Container(ft.Column(rows, spacing=2), padding=12,
                         bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=10),
            _add_btn('Ajouter un palier', add_palier, c),
            ft.Container(height=40),
        ]

    col.controls.extend(_build())
    return col


# ═══ CONFIG ═════════════════════════════════════════════════════════

def build_config_view(storage, t, on_save, on_toast, on_reload, on_theme_toggle, page=None):
    def c(k): return t.c(k)
    cfg = storage.cfg

    def field(label, value, hint='', password=False):
        lbl = _t(label, size=11, col=c('text2'))
        tf  = ft.TextField(
            value=value, hint_text=hint, password=password,
            can_reveal_password=password,
            bgcolor=c('card'), border_color=c('card_border'),
            focused_border_color=c('gold'), color=c('text'), text_size=13,
            content_padding=P.symmetric(horizontal=10, vertical=8),
        )
        return lbl, tf

    url_lbl, url_tf = field('URL WebDAV', cfg.get('dav_url',''),
                            'https://xxx/remote.php/dav/files/user/Oxy/')
    usr_lbl, usr_tf = field('Utilisateur', cfg.get('dav_user',''), 'ton@email.com')
    pw_lbl,  pw_tf  = field('Mot de passe', cfg.get('dav_pass',''), 'app password', password=True)
    status_txt = ft.Text('', size=12, font_family='DM Sans')

    def abtn(label, bg, on_click, danger=False):
        if danger:
            style = ft.ButtonStyle(bgcolor='transparent', color=c('danger'),
                                   side=ft.BorderSide(1, c('danger')),
                                   shape=ft.RoundedRectangleBorder(radius=8))
        else:
            fg = '#1a1a1a' if bg in ('gold','teal','green') else c('text')
            style = ft.ButtonStyle(bgcolor=c(bg), color=fg,
                                   shape=ft.RoundedRectangleBorder(radius=8),
                                   padding=P.symmetric(horizontal=16))
        return ft.ElevatedButton(label, on_click=on_click, height=38, style=style)

    def save_cfg(e):
        storage.save_config(url_tf.value.strip(), usr_tf.value.strip(), pw_tf.value)
        status_txt.value = 'Config sauvee'; status_txt.color = c('green')
        status_txt.update(); on_toast('Config sauvee')

    def test_cfg(e):
        storage.save_config(url_tf.value.strip(), usr_tf.value.strip(), pw_tf.value)
        ok, msg = storage.test_dav()
        status_txt.value = msg; status_txt.color = c('green') if ok else c('danger')
        status_txt.update()

    def clear_cfg(e):
        storage.clear_config()
        url_tf.value=''; usr_tf.value=''; pw_tf.value=''
        status_txt.value='Config effacee'; status_txt.color=c('text2')
        for w in [url_tf, usr_tf, pw_tf, status_txt]: w.update()

    import_status = ft.Text('', size=11, font_family='DM Sans')

    page_ref = [page]

    def do_export(e):
        import pathlib, datetime
        path = pathlib.Path.home() / f"oxycash-{datetime.date.today()}.json"
        path.write_text(storage.export_json(), encoding='utf-8')
        on_toast(f'Exporte: {path.name}')

    def do_import_pick(e):
        # page.run_task launches the async pick_files coroutine
        if page_ref[0] is None:
            on_toast('Import non disponible')
            return
        async def _pick_async():
            try:
                fp = ft.FilePicker()
                page_ref[0].overlay.append(fp)
                page_ref[0].update()
                files = await fp.pick_files(
                    dialog_title='Importer oxycash.json',
                    allowed_extensions=['json'],
                    file_type=ft.FilePickerFileType.CUSTOM,
                    with_data=True,
                )
                page_ref[0].overlay.remove(fp)
                page_ref[0].update()
                if not files:
                    return
                f = files[0]
                raw = (f.bytes.decode('utf-8') if f.bytes
                       else open(f.path, encoding='utf-8').read())
                ok = storage.import_json(raw)
                if ok:
                    import_status.value = f'Importe: {f.name}'
                    import_status.color = c('green')
                    try: import_status.update()
                    except: pass
                    on_reload()
                    on_toast(f'Importe: {f.name}')
                else:
                    import_status.value = 'Format invalide'
                    import_status.color = c('danger')
                    try: import_status.update()
                    except: pass
                    on_toast('Format JSON invalide')
            except Exception as ex:
                import_status.value = str(ex)[:60]
                import_status.color = c('danger')
                try: import_status.update()
                except: pass
                on_toast('Erreur import')
        page_ref[0].run_task(_pick_async)

    def do_reset(e):
        storage.reset(); on_reload(); on_toast('Reinitialise')

    def mk_card(title, *children):
        return ft.Container(
            ft.Column([_t(title, size=11, weight=ft.FontWeight.W_600, col=c('text3')),
                       ft.Container(height=6), *children], spacing=2),
            padding=14, bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=12,
        )

    return ft.Column([
        _t('Configuration', size=20, weight=ft.FontWeight.W_700,
           family='Playfair Display', col=c('text')),
        mk_card('WebDAV (Nextcloud, kDrive)',
                url_lbl, url_tf, ft.Container(height=2),
                usr_lbl, usr_tf, ft.Container(height=2),
                pw_lbl,  pw_tf,  ft.Container(height=6),
                ft.Row([abtn('Sauver','gold',save_cfg), abtn('Tester','teal',test_cfg),
                        abtn('Effacer','card',clear_cfg)], spacing=8),
                status_txt),
        mk_card('Import / Export',
                ft.Row([abtn('Exporter JSON','card',do_export),
                        abtn('Importer JSON','card',do_import_pick)], spacing=8),
                import_status),
        mk_card('Donnees', abtn('Reinitialiser','',do_reset,danger=True)),
        ft.Container(height=40),
    ], spacing=12, scroll=ft.ScrollMode.AUTO, expand=True)