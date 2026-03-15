"""Oxycash – Month view (Flet 0.82+)"""
from __future__ import annotations
import flet as ft
from typing import Callable
from core.model import Month, Line, Payment, fmt, fmt_sign, today, MONTHS
from core.i18n import T

P = ft.Padding
M = ft.Margin
BR = ft.BorderRadius
B = ft.Border
BS = ft.BorderSide

# SECTION_META: (label_key, color_key)
SECTION_META = {
    'revenus':   ('sec_income',      'teal'),
    'retraits':  ('sec_withdrawals', 'amber'),
    'fixes':     ('sec_fixed',       'blue'),
    'variables': ('sec_variable',    'gold'),
}
SECTION_ICONS = {
    'revenus': '💰', 'retraits': '🏧', 'fixes': '📌', 'variables': '🔄',
}


def build_month_view(month_key, month: Month, t, on_save: Callable, on_toast: Callable, all_months: dict = None, page=None, frais: dict = None) -> ft.Column:

    def c(k): return t.c(k)
    expanded: dict[str, bool] = {}
    if all_months is None: all_months = {}
    page_ref = [page]  # mutable ref for dialog

    storage_frais = [frais]  # mutable ref so _sync_frais sees updates

    def _sync_frais(sec_key, idx):
        from core.model import sync_frais_from_line
        if storage_frais[0] is None:
            return
        line = month.section(sec_key)[idx]
        class _D:
            months = all_months
            frais  = storage_frais[0]
        sync_frais_from_line(_D(), month_key, sec_key, line)

    # ── widget helpers ──────────────────────────────────────────────────────

    def txt(s, size=12, weight=ft.FontWeight.NORMAL, col='text', family='DM Sans',
            align=ft.TextAlign.LEFT, expand=False, overflow=None, no_wrap=False):
        kw = dict(size=max(6, size + t.scale), weight=weight, font_family=family,
                  color=c(col), text_align=align)
        if expand:   kw['expand']   = True
        if overflow: kw['overflow'] = overflow
        if no_wrap:  kw['no_wrap']  = True
        return ft.Text(str(s), **kw)

    def num_field(value, on_change, width=60, col='text'):
        return ft.TextField(
            value=value, width=width, height=34,
            text_size=12, text_align=ft.TextAlign.RIGHT,
            color=c(col), bgcolor='transparent',
            border_color=c('card_border'),
            focused_border_color=c('gold'),
            content_padding=P.symmetric(horizontal=6, vertical=2),
            on_change=on_change,
            keyboard_type=ft.KeyboardType.NUMBER,
        )

    def ro_field(value, width=60, col='text'):
        return ft.TextField(
            value=value, width=width, height=34,
            text_size=12, text_align=ft.TextAlign.RIGHT,
            color=c(col), bgcolor='transparent',
            border_color='transparent', read_only=True,
            content_padding=P.symmetric(horizontal=6, vertical=2),
        )

    def btn(label, bg_col, on_click, fg='#1a1a1a'):
        return ft.ElevatedButton(
            label, on_click=on_click, height=34,
            style=ft.ButtonStyle(
                bgcolor=c(bg_col), color=fg,
                shape=ft.RoundedRectangleBorder(radius=6),
                padding=P.symmetric(horizontal=12),
            ),
        )

    # ── summary (6 cartes + bar chart — identique au HTML) ──────────────────

    def summary() -> ft.Column:
        # Revenus
        rev_banque = sum(l.banque for l in month.revenus)
        rev_cash   = sum(l.cash   for l in month.revenus)
        rev_total  = rev_banque + rev_cash
        rev_recu_b = sum(l.etat() for l in month.revenus if l.banque > 0.01)
        rev_recu_c = sum(l.etat() for l in month.revenus if l.cash   > 0.01)

        # Retraits
        ret_a_retirer = sum(l.banque for l in month.retraits)
        ret_retire    = sum(l.etat() for l in month.retraits)
        ret_solde     = ret_a_retirer - ret_retire

        # Dépenses (fixes + variables)
        all_dep    = month.fixes + month.variables
        dep_banque = sum(l.banque for l in all_dep)
        dep_cash   = sum(l.cash   for l in all_dep)

        # Payé
        paye_banque = sum(l.etat() for l in all_dep if l.banque > 0.01) + ret_retire
        paye_cash   = sum(l.etat() for l in all_dep if l.cash   > 0.01)
        paye_total  = paye_banque + paye_cash

        # À payer
        a_payer_b = dep_banque - sum(l.etat() for l in all_dep if l.banque > 0.01)
        a_payer_c = dep_cash   - sum(l.etat() for l in all_dep if l.cash   > 0.01)
        a_payer_t = a_payer_b + a_payer_c

        # Prévision (si tout payé)
        prev_banque = rev_banque - dep_banque - ret_a_retirer
        prev_cash   = ret_a_retirer - dep_cash
        prev_total  = prev_banque + prev_cash

        # Solde réel
        solde_banque = rev_recu_b - paye_banque
        solde_cash   = ret_retire  - paye_cash
        solde_total  = solde_banque + solde_cash

        def cc(n): return 'green' if n > 0.01 else ('danger' if n < -0.01 else 'text')

        def card(label, rows_data, total_val, total_col, big=False) -> ft.Container:
            rows = []
            for lbl, val, col in rows_data:
                rows.append(ft.Row([
                    txt(lbl, size=11, col='text2'),
                    txt(fmt(val), size=12, weight=ft.FontWeight.W_700, col=col,
                        align=ft.TextAlign.RIGHT),
                ], alignment=ft.MainAxisAlignment.SPACE_BETWEEN))
            # total row
            total_size = 18 if big else 12
            total_w    = ft.FontWeight.W_700
            rows.append(ft.Container(
                ft.Row([
                    txt('Total', size=11, weight=ft.FontWeight.W_600, col='text2'),
                    txt(fmt(total_val) if not big else fmt_sign(total_val),
                        size=total_size, weight=total_w, col=total_col,
                        align=ft.TextAlign.RIGHT, family='Playfair Display' if big else 'DM Sans'),
                ], alignment=ft.MainAxisAlignment.SPACE_BETWEEN),
                border=B.only(top=BS(1, c('card_border'))),
                padding=P.only(top=4), margin=M.only(top=4),
            ))
            return ft.Container(
                ft.Column([
                    txt(label, size=9, weight=ft.FontWeight.W_600, col='text3'),
                    ft.Container(height=4),
                    *rows,
                ], spacing=4),
                expand=True, padding=12,
                bgcolor=c('card'),
                border=B.all(1, c('card_border')), border_radius=10,
            )

        grid = ft.Column([
            ft.Row([
                card(T['card_income'], [
                    ('Banque', rev_banque, 'text'),
                    ('Cash',   rev_cash,   'text'),
                ], rev_total, 'green'),
                card(T['card_withdrawals'], [
                    (T['card_to_withdraw'], ret_a_retirer, 'text'),
                    (T['card_withdrawn'],    ret_retire,    'text'),
                ], ret_solde, cc(-ret_solde)),
            ], spacing=8),
            ft.Row([
                card(T['card_paid'], [
                    ('Banque', paye_banque, 'text'),
                    ('Cash',   paye_cash,   'text'),
                ], paye_total, 'teal'),
                card(T['card_to_pay'], [
                    ('Banque', a_payer_b, 'text'),
                    ('Cash',   a_payer_c, 'text'),
                ], a_payer_t, 'amber'),
            ], spacing=8),
            ft.Row([
                card(T['card_forecast'], [
                    ('Banque', prev_banque, cc(prev_banque)),
                    ('Cash',   prev_cash,   cc(prev_cash)),
                ], prev_total, cc(prev_total), big=True),
                card(T['card_balance'], [
                    ('Banque', solde_banque, cc(solde_banque)),
                    ('Cash',   solde_cash,   cc(solde_cash)),
                ], solde_total, cc(solde_total), big=True),
            ], spacing=8),
        ], spacing=8)

        # Bar chart — Budget vs Payé (Retraits / Fixes / Variables)
        fix_budget = sum(l.banque + l.cash for l in month.fixes)
        var_budget = sum(l.banque + l.cash for l in month.variables)
        fix_paye   = sum(l.etat() for l in month.fixes)
        var_paye   = sum(l.etat() for l in month.variables)

        cats = [
            (T['chart_withdrawals'],  ret_a_retirer, ret_retire,  'teal'),
            (T['chart_fixed'],     fix_budget,    fix_paye,    'gold'),
            (T['chart_variable'], var_budget,    var_paye,    'red'),
        ]
        max_v = max((b for _, b, _, _ in cats), default=1) or 1
        BAR_H = 75

        bar_cols = []
        for name, budget, paye, col_key in cats:
            bh = max(2, budget / max_v * BAR_H)
            ph = max(2, paye   / max_v * BAR_H)
            bar_cols.append(ft.Column([
                txt(fmt(budget), size=9, weight=ft.FontWeight.W_600,
                    col=col_key, align=ft.TextAlign.CENTER),
                ft.Row([
                    ft.Container(width=18, height=bh,
                                 bgcolor=c(col_key), opacity=0.3,
                                 border_radius=BR.only(top_left=3, top_right=3)),
                    ft.Container(width=18, height=ph,
                                 bgcolor=c(col_key),
                                 border_radius=BR.only(top_left=3, top_right=3)),
                ], spacing=2, alignment=ft.MainAxisAlignment.CENTER,
                   vertical_alignment=ft.CrossAxisAlignment.END),
                txt(name, size=9, col='text3', align=ft.TextAlign.CENTER),
            ], spacing=4, horizontal_alignment=ft.CrossAxisAlignment.CENTER, expand=True))

        chart = ft.Container(
            ft.Column([
                txt(T['card_budget_vs'], size=9, weight=ft.FontWeight.W_600, col='text3'),
                ft.Container(height=4),
                ft.Row(bar_cols, alignment=ft.MainAxisAlignment.SPACE_AROUND,
                       vertical_alignment=ft.CrossAxisAlignment.END),
                ft.Container(height=4),
                ft.Row([
                    txt(T['card_budget'], size=9, col='text3'),
                    txt(T['card_paid_lbl'],   size=9, col='text3'),
                ], alignment=ft.MainAxisAlignment.CENTER, spacing=16),
            ], spacing=0),
            padding=14, bgcolor=c('card'),
            border=B.all(1, c('card_border')), border_radius=10,
        )

        return ft.Column([grid, chart], spacing=8)

    # ── sub-payment panel ────────────────────────────────────────────────────

    def sub_panel(sec_key, li, line: Line, rebuild):
        pays = sorted(line.payments, key=lambda p: p.date, reverse=True)
        rows = []
        for pi, p in enumerate(pays):
            def del_pay(e, _pi=pi, _sec=sec_key, _li=li):
                month.section(_sec)[_li].payments.pop(_pi)
                on_save(); rebuild()
                on_toast(T['pay_deleted'])
            rows.append(ft.Row([
                txt(p.date[5:].replace('-', '/'), size=11, col='text3'),
                ft.Container(expand=True),
                txt(f"{fmt(p.amount)} CHF", size=12, weight=ft.FontWeight.W_600,
                    col='green' if p.amount >= 0 else 'danger'),
                ft.IconButton(ft.Icons.CLOSE, icon_size=14, icon_color=c('danger'),
                              on_click=del_pay,
                              style=ft.ButtonStyle(padding=P.all(2))),
            ], vertical_alignment=ft.CrossAxisAlignment.CENTER, spacing=4))

        if not rows:
            rows.append(txt(T['no_payments'], size=11, col='text3'))

        date_f = ft.TextField(value=today(), width=110, height=30, text_size=11,
                              bgcolor='transparent', border_color=c('card_border'),
                              focused_border_color=c('gold'),
                              content_padding=P.symmetric(horizontal=6, vertical=2),
                              hint_text=T['pay_date_hint'])
        amt_f  = ft.TextField(value='', width=80, height=30, text_size=12,
                              text_align=ft.TextAlign.RIGHT,
                              bgcolor='transparent', border_color=c('card_border'),
                              focused_border_color=c('gold'),
                              content_padding=P.symmetric(horizontal=6, vertical=2),
                              hint_text=T['pay_amt_hint'], keyboard_type=ft.KeyboardType.NUMBER)

        def add_pay(e, _sec=sec_key, _li=li):
            try:
                amount = float(amt_f.value.replace(',', '.'))
                date   = date_f.value.strip() or today()
                month.section(_sec)[_li].payments.append(Payment(date, amount))
                on_save(); rebuild()
                on_toast(f"+{fmt(amount)} CHF")
            except ValueError:
                on_toast(T['pay_invalid'])

        add_row = ft.Row([
            date_f, amt_f,
            ft.ElevatedButton('+', on_click=add_pay, height=30,
                              style=ft.ButtonStyle(bgcolor=c('teal'), color='#1a1a1a',
                                                   padding=P.symmetric(horizontal=12),
                                                   shape=ft.RoundedRectangleBorder(radius=6))),
        ], spacing=6)

        return ft.Container(
            ft.Column(rows + [ft.Divider(height=1, color=c('card_border')), add_row], spacing=4),
            padding=P.only(left=12, right=12, top=8, bottom=10),
            bgcolor=c('card'),
            border=B.only(
                left=BS(1, c('card_border')),
                right=BS(1, c('card_border')),
                bottom=BS(1, c('card_border')),
            ),
            border_radius=BR.only(bottom_left=10, bottom_right=10),
        )

    # ── line item ────────────────────────────────────────────────────────────

    def line_row(sec_key, idx, line: Line, rebuild):
        key     = f"{month_key}-{sec_key}-{idx}"
        is_exp  = expanded.get(key, False)
        etat    = line.etat()
        solde   = line.solde()
        is_var  = sec_key == 'variables'
        n_pay   = len(line.payments)
        sc      = 'green' if abs(solde) < 0.01 else ('danger' if solde > 0.01 else 'green')

        def toggle(e):
            expanded[key] = not expanded.get(key, False)
            rebuild()

        def upd_banque(e, _s=sec_key, _i=idx):
            try:
                month.section(_s)[_i].banque = float(e.control.value.replace(',','.'))
                _sync_frais(_s, _i)
                on_save(); rebuild()
            except ValueError: pass

        def upd_cash(e, _s=sec_key, _i=idx):
            try:
                month.section(_s)[_i].cash = float(e.control.value.replace(',','.'))
                _sync_frais(_s, _i)
                on_save(); rebuild()
            except ValueError: pass

        def upd_name(e, _s=sec_key, _i=idx):
            month.section(_s)[_i].name = e.control.value; on_save()

        def del_var(e, _s=sec_key, _i=idx):
            month.section(_s).pop(_i); on_save(); rebuild(); on_toast(T['toast_deleted'])

        # name widget
        if is_var:
            name_w = ft.TextField(
                value=line.name, expand=True, height=34, text_size=12,
                bgcolor='transparent', border_color='transparent',
                content_padding=P.symmetric(horizontal=0, vertical=2),
                on_blur=upd_name, color=c('text'),
            )
        else:
            name_w = ft.Row([
                txt(line.name, size=12, weight=ft.FontWeight.W_500, col='text',
                    expand=True, overflow=ft.TextOverflow.ELLIPSIS, no_wrap=True),
            ], spacing=4, expand=True)

        def open_recurring_dialog(e, _s=sec_key, _i=idx):
            from core.model import apply_recurring
            if page_ref[0] is None:
                on_toast(T['rec_na'])
                return

            src = month.section(_s)[_i]
            freq_state = [(src.recurring or {}).get('freq', 3)]

            FREQ_OPTIONS = [
                (T['rec_every_1'],  1),
                (T['rec_every_2'],  2),
                (T['rec_every_3'],  3),
                (T['rec_every_6'],  6),
                (T['rec_every_12'], 12),
            ]

            # RadioGroup is simplest for single-select in Flet
            rg = ft.RadioGroup(
                value=str(freq_state[0]),
                content=ft.Column([
                    ft.Radio(value=str(v), label=lbl,
                             label_style=ft.TextStyle(
                                 color=c('text'), size=13,
                                 font_family='DM Sans'))
                    for lbl, v in FREQ_OPTIONS
                ], spacing=2),
                on_change=lambda e2: freq_state.__setitem__(0, int(e2.control.value)),
            )

            dlg = ft.AlertDialog(
                modal=True,
                bgcolor=c('bg2'),
                title=ft.Text(
                    f"{T['rec_title']}  {src.name}",
                    font_family='Playfair Display', size=15,
                    weight=ft.FontWeight.W_600, color=c('text'),
                ),
                content=ft.Container(
                    ft.Column([
                        ft.Text(T['rec_frequency'], size=11,
                                color=c('text2'), font_family='DM Sans'),
                        ft.Container(height=6),
                        rg,
                    ], spacing=0),
                    width=260, padding=P.only(top=4),
                ),
                actions=[
                    ft.TextButton(
                        T['rec_cancel'],
                        on_click=lambda e2: page_ref[0].pop_dialog(),
                        style=ft.ButtonStyle(color=c('text2')),
                    ),
                    ft.TextButton(
                        T['rec_disable'],
                        on_click=lambda e2: _remove_rec(_s, _i),
                        style=ft.ButtonStyle(color=c('danger')),
                    ),
                    ft.ElevatedButton(
                        T['rec_apply'],
                        on_click=lambda e2: _apply_rec(_s, _i, freq_state[0]),
                        style=ft.ButtonStyle(
                            bgcolor=c('teal'), color='#1a1a1a',
                            shape=ft.RoundedRectangleBorder(radius=8),
                        ),
                    ),
                ],
                actions_alignment=ft.MainAxisAlignment.END,
            )
            page_ref[0].show_dialog(dlg)

        def _apply_rec(_s, _i, freq):
            from core.model import apply_recurring
            src2 = month.section(_s)[_i]
            src2.recurring = {'freq': freq, 'start': month_key}
            # propagate to all months
            class _D:
                months = all_months
            apply_recurring(_D())
            on_save()
            page_ref[0].pop_dialog()
            on_toast(T.fmt('rec_active', n=freq))
            rebuild()

        def _remove_rec(_s, _i):
            month.section(_s)[_i].recurring = None
            on_save()
            page_ref[0].pop_dialog()
            on_toast(T['rec_disabled'])
            rebuild()

        # Récurrence disponible sur toutes les sections
        trailing = [
            ft.IconButton(
                ft.Icons.REPLAY, icon_size=14,
                icon_color=c('teal') if line.recurring else c('text3'),
                tooltip='Récurrence',
                on_click=open_recurring_dialog,
                style=ft.ButtonStyle(padding=P.all(2)),
            )
        ]
        # Suppression uniquement pour les variables
        if is_var:
            trailing.append(ft.IconButton(
                ft.Icons.DELETE_OUTLINE, icon_size=16, icon_color=c('danger'),
                on_click=del_var,
                style=ft.ButtonStyle(padding=P.all(2)),
            ))

        row_content = ft.Row([
            name_w,
            num_field(fmt(line.banque), upd_banque, col='blue'),
            num_field(fmt(line.cash),   upd_cash,   col='amber'),
            ro_field(fmt(etat),   col='teal'),
            ro_field(fmt(solde),  col=sc, width=54),
            *trailing,
        ], spacing=4, vertical_alignment=ft.CrossAxisAlignment.CENTER)

        header = ft.GestureDetector(
            content=ft.Container(
                row_content,
                padding=P.symmetric(horizontal=12, vertical=8),
                bgcolor=c('card'),
                border=B.all(1, c('gold') if is_exp else c('card_border')),
                border_radius=BR.only(
                    top_left=10, top_right=10,
                    bottom_left=0 if is_exp else 10,
                    bottom_right=0 if is_exp else 10,
                ),
            ),
            on_tap=toggle,
        )

        parts = [header]
        if is_exp:
            parts.append(sub_panel(sec_key, idx, line, rebuild))
        return ft.Column(parts, spacing=0)

    # ── section ──────────────────────────────────────────────────────────────

    def section(sec_key) -> ft.Column:
        lbl_key, col_key = SECTION_META[sec_key]; icon = SECTION_ICONS[sec_key]; label = T[lbl_key]
        col_ref = ft.Column([], spacing=4)

        def rebuild():
            col_ref.controls.clear()
            col_ref.controls.extend(section_body(sec_key))
            col_ref.update()

        def section_body(sk):
            lines    = month.section(sk)
            tot_b    = sum(l.banque + l.cash for l in lines)
            tot_etat = sum(l.etat() for l in lines)
            tot_sol  = sum(l.solde() for l in lines)
            is_open  = expanded.get(f'sec-{month_key}-{sk}', False)
            lbl2_key, ck2 = SECTION_META[sk]; icon2 = SECTION_ICONS[sk]; lbl2 = T[lbl2_key]

            def toggle_sec(e):
                expanded[f'sec-{month_key}-{sk}'] = not is_open
                rebuild()

            header = ft.GestureDetector(
                content=ft.Container(
                    ft.Row([
                        txt(icon2, size=16, col='text'),
                        txt(lbl2, size=13, weight=ft.FontWeight.W_700,
                            family='Playfair Display', col='text', expand=True),
                        txt(f"{fmt(tot_b)} CHF", size=12, weight=ft.FontWeight.W_700, col=ck2),
                        ft.Icon(ft.Icons.EXPAND_MORE if is_open else ft.Icons.CHEVRON_RIGHT,
                                size=16, color=c('text3')),
                    ], spacing=8, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                    padding=P.symmetric(vertical=10),
                ),
                on_tap=toggle_sec,
            )

            parts = [header]
            if is_open:
                # legend
                parts.append(ft.Row([
                    ft.Container(expand=True),
                    ft.Container(txt(T['col_bank'], size=9, col='blue', weight=ft.FontWeight.W_600, align=ft.TextAlign.CENTER), width=60),
                    ft.Container(txt(T['col_cash'],   size=9, col='amber', weight=ft.FontWeight.W_600, align=ft.TextAlign.CENTER), width=60),
                    ft.Container(txt(T['col_state'],   size=9, col='teal', weight=ft.FontWeight.W_600, align=ft.TextAlign.CENTER), width=60),
                    ft.Container(txt(T['col_balance'],  size=9, col='gold', weight=ft.FontWeight.W_600, align=ft.TextAlign.CENTER), width=54),
                ], spacing=4))

                for i, ln in enumerate(month.section(sk)):
                    parts.append(line_row(sk, i, ln, rebuild))

                # add row for all sections
                _ADD_LABELS = {
                    'revenus':   'add_income',
                    'retraits':  'add_withdrawal',
                    'fixes':     'add_fixed',
                    'variables': 'add_expense',
                }
                def make_add(sk2=sk):
                    def _add(e):
                        from core.model import mk_line as _mk
                        month.section(sk2).append(_mk('New' if True else ''))
                        on_save(); rebuild(); on_toast(T['toast_added'])
                    return _add
                parts.append(ft.GestureDetector(
                    content=ft.Container(
                        ft.Row([ft.Icon(ft.Icons.ADD, size=14, color=c('text3')),
                                txt(T[_ADD_LABELS[sk]], size=12, col='text3')], spacing=6),
                        padding=P.symmetric(horizontal=12, vertical=8),
                        border=B.all(1, c('card_border')), border_radius=10,
                        margin=M.only(top=4),
                    ),
                    on_tap=make_add(),
                ))

                # section total
                sc2 = 'green' if tot_sol >= 0 else 'danger'
                parts.append(ft.Container(
                    ft.Row([
                        txt(T['col_total'], size=11, weight=ft.FontWeight.W_700, col='text2', expand=True),
                        ft.Container(txt(fmt(tot_b),    size=11, weight=ft.FontWeight.W_700, col=ck2,    align=ft.TextAlign.RIGHT), width=60),
                        ft.Container(width=60),
                        ft.Container(txt(fmt(tot_etat), size=11, weight=ft.FontWeight.W_700, col='teal', align=ft.TextAlign.RIGHT), width=60),
                        ft.Container(txt(fmt(tot_sol),  size=11, weight=ft.FontWeight.W_700, col=sc2,    align=ft.TextAlign.RIGHT), width=54),
                    ], spacing=4),
                    padding=P.symmetric(horizontal=12, vertical=6),
                    border=B.only(top=BS(1, c('card_border'))),
                    margin=M.only(top=4),
                ))
            return parts

        col_ref.controls.extend(section_body(sec_key))
        return col_ref

    # ── registre view ────────────────────────────────────────────────────────

    SECTION_COLORS = {
        'revenus':   'teal',
        'retraits':  'amber',
        'fixes':     'blue',
        'variables': 'gold',
    }
    SECTION_LABELS = {
        'revenus':   'sec_income',
        'retraits':  'sec_withdrawals',
        'fixes':     'sec_fixed',
        'variables': 'sec_variable',
    }

    def build_registre(sort_asc: list, root_col: ft.Column):
        """Return controls for the registre (ledger) view."""

        # Collect all payments across all sections
        entries = []
        for sec_key in ('revenus', 'retraits', 'fixes', 'variables'):
            for li, line in enumerate(month.section(sec_key)):
                for pi, p in enumerate(line.payments):
                    entries.append({
                        'date':    p.date,
                        'amount':  p.amount,
                        'name':    line.name,
                        'sec':     sec_key,
                        'li':      li,
                        'pi':      pi,
                    })

        entries.sort(key=lambda x: x['date'], reverse=not sort_asc[0])

        def del_entry(e, entry=None):
            month.section(entry['sec'])[entry['li']].payments.pop(entry['pi'])
            on_save()
            refresh_registre(sort_asc, root_col)
            on_toast(T['toast_deleted'])

        rows = []
        prev_date = None
        for entry in entries:
            # Date separator
            if entry['date'] != prev_date:
                prev_date = entry['date']
                rows.append(ft.Container(
                    ft.Text(entry['date'], size=10, color=c('text3'),
                            font_family='DM Sans', weight=ft.FontWeight.W_600),
                    padding=P.only(top=8, bottom=2, left=4),
                ))

            col_key = SECTION_COLORS[entry['sec']]
            sign    = '+' if entry['sec'] == 'revenus' else '-'
            amt_col = 'green' if entry['sec'] == 'revenus' else 'danger'

            rows.append(ft.Container(
                ft.Row([
                    ft.Container(
                        ft.Text(T[SECTION_LABELS[entry['sec']]], size=9,
                                color=c(col_key), font_family='DM Sans',
                                weight=ft.FontWeight.W_600),
                        width=56,
                    ),
                    ft.Text(entry['name'], size=12, color=c('text'),
                            font_family='DM Sans', expand=True,
                            overflow=ft.TextOverflow.ELLIPSIS, no_wrap=True),
                    ft.Text(f"{sign}{fmt(entry['amount'])} CHF", size=13,
                            color=c(amt_col), font_family='DM Sans',
                            weight=ft.FontWeight.W_700),
                    ft.IconButton(
                        ft.Icons.DELETE_OUTLINE, icon_size=14,
                        icon_color=c('danger'),
                        on_click=lambda e, en=entry: del_entry(e, en),
                        style=ft.ButtonStyle(padding=P.all(2)),
                    ),
                ], spacing=8, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                padding=P.symmetric(horizontal=12, vertical=8),
                bgcolor=c('card'),
                border=B.all(1, c('card_border')),
                border_radius=8,
            ))

        if not rows:
            rows.append(ft.Container(
                ft.Text('Aucune transaction ce mois', size=13,
                        color=c('text3'), font_family='DM Sans',
                        text_align=ft.TextAlign.CENTER),
                padding=P.symmetric(vertical=32),
                alignment=ft.Alignment(0, 0),
            ))

        # ── Add entry form ──
        date_f = ft.TextField(
            value=today(), width=120, height=34, text_size=12,
            bgcolor='transparent', border_color=c('card_border'),
            focused_border_color=c('gold'),
            content_padding=P.symmetric(horizontal=8, vertical=4),
            hint_text=T['pay_date_hint'], color=c('text'),
        )
        amt_f = ft.TextField(
            value='', width=90, height=34, text_size=12,
            text_align=ft.TextAlign.RIGHT,
            bgcolor='transparent', border_color=c('card_border'),
            focused_border_color=c('gold'),
            content_padding=P.symmetric(horizontal=8, vertical=4),
            hint_text=T['pay_amt_hint'], keyboard_type=ft.KeyboardType.NUMBER,
            color=c('text'),
        )
        name_f = ft.TextField(
            value='', height=34, text_size=12, expand=True,
            bgcolor='transparent', border_color=c('card_border'),
            focused_border_color=c('gold'),
            content_padding=P.symmetric(horizontal=8, vertical=4),
            hint_text='Libelle', color=c('text'),
        )

        SEC_OPTIONS = [
            (T['reg_variable'], 'variables'),
            (T['reg_fixed'],     'fixes'),
            (T['reg_withdrawal'],  'retraits'),
            (T['reg_income'],   'revenus'),
        ]
        sec_state = ['variables']

        sec_btns_col = ft.Row([], spacing=4)

        def render_sec_btns():
            sec_btns_col.controls.clear()
            for lbl, key in SEC_OPTIONS:
                is_sel = sec_state[0] == key
                ck = SECTION_COLORS[key]
                def make_sel(k):
                    def _s(e):
                        sec_state[0] = k
                        render_sec_btns()
                        try: sec_btns_col.update()
                        except: pass
                    return _s
                sec_btns_col.controls.append(ft.GestureDetector(
                    content=ft.Container(
                        ft.Text(lbl, size=10, font_family='DM Sans',
                                color='#1a1a1a' if is_sel else c(ck),
                                weight=ft.FontWeight.W_600),
                        padding=P.symmetric(horizontal=8, vertical=5),
                        bgcolor=c(ck) if is_sel else 'transparent',
                        border=B.all(1, c(ck)),
                        border_radius=6,
                    ),
                    on_tap=make_sel(key),
                ))

        render_sec_btns()

        def add_entry(e):
            try:
                amount = float(amt_f.value.replace(',', '.'))
                date   = date_f.value.strip() or today()
                name   = name_f.value.strip()
                sec    = sec_state[0]
                if not name:
                    on_toast(T['toast_label_req'])
                    return
                # find or create line with that name in the section
                lines  = month.section(sec)
                target = next((l for l in lines if l.name == name), None)
                if target is None:
                    from core.model import mk_line
                    new_line = mk_line(name, banque=amount if sec != 'variables' else 0,
                                       cash=0 if sec != 'variables' else amount)
                    lines.append(new_line)
                    target = lines[-1]
                target.payments.append(Payment(date, amount))
                amt_f.value = ''
                name_f.value = ''
                on_save()
                refresh_registre(sort_asc, root_col)
                on_toast(f'+{fmt(amount)} CHF → {name}')
            except ValueError:
                on_toast(T['toast_invalid'])

        add_form = ft.Container(
            ft.Column([
                ft.Text(T['reg_new_tx'], size=10, color=c('text3'),
                        font_family='DM Sans', weight=ft.FontWeight.W_600),
                ft.Container(height=4),
                sec_btns_col,
                ft.Container(height=6),
                ft.Row([date_f, amt_f], spacing=6),
                ft.Container(height=4),
                ft.Row([
                    name_f,
                    ft.ElevatedButton(
                        '+ Ajouter', on_click=add_entry, height=34,
                        style=ft.ButtonStyle(
                            bgcolor=c('gold'), color='#1a1a1a',
                            shape=ft.RoundedRectangleBorder(radius=8),
                            padding=P.symmetric(horizontal=14),
                        ),
                    ),
                ], spacing=6),
            ], spacing=0),
            padding=14, bgcolor=c('card'),
            border=B.all(1, c('card_border')), border_radius=10,
        )

        return [summary(), add_form, *rows, ft.Container(height=40)]

    def refresh_registre(sort_asc, root_col):
        root_col.controls.clear()
        root_col.controls.extend(_registre_header(sort_asc, root_col))
        root_col.controls.extend(build_registre(sort_asc, root_col))
        try: root_col.update()
        except: pass

    def _registre_header(sort_asc, root_col):
        sort_label = T['reg_date_asc'] if sort_asc[0] else T['reg_date_desc']
        def toggle_sort(e):
            sort_asc[0] = not sort_asc[0]
            refresh_registre(sort_asc, root_col)
        return [ft.GestureDetector(
            content=ft.Container(
                ft.Text(sort_label, size=11, color=c('teal'),
                        font_family='DM Sans', weight=ft.FontWeight.W_600),
                padding=P.symmetric(horizontal=10, vertical=6),
                border=B.all(1, c('teal')), border_radius=8,
                alignment=ft.Alignment(0, 0),
            ),
            on_tap=toggle_sort,
        )]

    # ── assemble ─────────────────────────────────────────────────────────────

    view_mode   = ['dashboard']  # 'dashboard' | 'registre'
    sort_asc    = [False]        # descending by default
    main_col    = ft.Column([], spacing=8, scroll=ft.ScrollMode.AUTO, expand=True)

    def toggle_view(e):
        view_mode[0] = 'registre' if view_mode[0] == 'dashboard' else 'dashboard'
        refresh_main()

    def refresh_main():
        main_col.controls.clear()
        # top bar: toggle button
        lbl = T['reg_title'] if view_mode[0] == 'dashboard' else T['dash_title']
        toggle_btn = ft.GestureDetector(
            content=ft.Container(
                ft.Text(lbl, size=11, color=c('gold'),
                        font_family='DM Sans', weight=ft.FontWeight.W_600),
                padding=P.symmetric(horizontal=12, vertical=6),
                border=B.all(1, c('gold')), border_radius=8,
            ),
            on_tap=toggle_view,
        )
        main_col.controls.append(
            ft.Container(toggle_btn, padding=P.only(bottom=4))
        )
        if view_mode[0] == 'dashboard':
            main_col.controls += [
                summary(),
                ft.Divider(height=1, color='transparent'),
                section('revenus'),
                section('retraits'),
                section('fixes'),
                section('variables'),
                ft.Container(height=40),
            ]
        else:
            main_col.controls += _registre_header(sort_asc, main_col)
            main_col.controls += build_registre(sort_asc, main_col)
        try: main_col.update()
        except: pass

    refresh_main()
    return main_col