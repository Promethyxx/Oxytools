"""Oxycash – Annual charts view (Flet 0.82+)"""
from __future__ import annotations
import flet as ft
from core.model import AppData, MONTHS, fmt
from core.i18n import T

P  = ft.Padding
M  = ft.Margin
B  = ft.Border
BS = ft.BorderSide
BR = ft.BorderRadius


def build_charts_view(data: AppData, t, currency: str) -> ft.Column:
    def c(k): return t.c(k)
    sc = t.scale

    def txt(s, size=12, weight=ft.FontWeight.NORMAL, col='text',
            family='DM Sans', align=ft.TextAlign.LEFT, expand=False):
        return ft.Text(str(s), size=max(6, size + sc), weight=weight,
                       font_family=family, color=c(col),
                       text_align=align, expand=expand if expand else None)

    # ── Compute monthly data ─────────────────────────────────────────────────
    month_labels = T.months_short
    revenues  = []
    expenses  = []
    balances  = []
    cumul     = 0.0

    for mk in MONTHS:
        m = data.months.get(mk)
        if m is None:
            revenues.append(0); expenses.append(0); balances.append(cumul)
            continue
        # Use budgeted amounts (banque+cash), not just payments received
        rev_budget = sum(l.banque + l.cash for l in m.revenus)
        rev_paid   = sum(l.etat() for l in m.revenus)
        exp_budget = sum(l.banque + l.cash for l in m.fixes + m.variables) + sum(l.banque + l.cash for l in m.retraits)
        exp_paid   = sum(l.etat() for l in m.fixes + m.variables) + sum(l.etat() for l in m.retraits)
        # Show whichever is greater: budget or actual payments
        rev = max(rev_budget, rev_paid)
        exp = max(exp_budget, exp_paid)
        revenues.append(rev)
        expenses.append(exp)
        cumul += rev - exp
        balances.append(cumul)

    max_bar   = max(max(revenues), max(expenses), 1)
    BAR_H     = 120
    BAR_W     = 18
    COL_W     = 44

    # ── Bar chart: revenues vs expenses ──────────────────────────────────────
    def bar_chart() -> ft.Container:
        cols = []
        for i, mk in enumerate(MONTHS):
            rev_h = max(2, revenues[i] / max_bar * BAR_H)
            exp_h = max(2, expenses[i] / max_bar * BAR_H)
            cols.append(ft.Column([
                ft.Row([
                    ft.Container(width=BAR_W, height=rev_h,
                                 bgcolor=c('teal'), opacity=0.85,
                                 border_radius=BR.only(top_left=3, top_right=3)),
                    ft.Container(width=BAR_W, height=exp_h,
                                 bgcolor=c('danger'), opacity=0.75,
                                 border_radius=BR.only(top_left=3, top_right=3)),
                ], spacing=2, alignment=ft.MainAxisAlignment.CENTER,
                   vertical_alignment=ft.CrossAxisAlignment.END),
                txt(month_labels[i], size=9, col='text3',
                    align=ft.TextAlign.CENTER),
            ], spacing=4, horizontal_alignment=ft.CrossAxisAlignment.CENTER,
               width=COL_W))

        legend = ft.Row([
            ft.Row([ft.Container(width=10, height=10, bgcolor=c('teal'),
                                 border_radius=2),
                    txt(T['card_income'], size=10, col='text2')], spacing=4),
            ft.Row([ft.Container(width=10, height=10, bgcolor=c('danger'),
                                 border_radius=2),
                    txt(T['chart_fixed'] + ' + ' + T['chart_variable'],
                        size=10, col='text2')], spacing=4),
        ], spacing=16, alignment=ft.MainAxisAlignment.CENTER)

        return ft.Container(
            ft.Column([
                txt(T['chart_income_vs_exp'], size=11,
                    weight=ft.FontWeight.W_600, col='text3'),
                ft.Container(height=6),
                ft.Row(cols, alignment=ft.MainAxisAlignment.SPACE_BETWEEN,
                       vertical_alignment=ft.CrossAxisAlignment.END),
                ft.Container(height=8),
                legend,
            ], spacing=0),
            padding=14, bgcolor=c('card'),
            border=B.all(1, c('card_border')), border_radius=12,
        )

    # ── Balance curve (sparkline-style SVG) ──────────────────────────────────
    def balance_chart() -> ft.Container:
        W, H   = 320, 100
        min_b  = min(balances)
        max_b  = max(balances) if max(balances) != min_b else min_b + 1
        rng    = max_b - min_b or 1

        def px(i, val):
            x = int(i / 11 * (W - 20)) + 10
            y = int(H - 10 - (val - min_b) / rng * (H - 20))
            return x, y

        pts = [px(i, v) for i, v in enumerate(balances)]

        # zero line
        zero_y = int(H - 10 - (0 - min_b) / rng * (H - 20))
        zero_y = max(5, min(H - 5, zero_y))

        # build polyline
        polyline = ' '.join(f"{x},{y}" for x, y in pts)

        # colored dots
        dots = ''
        for i, (x, y) in enumerate(pts):
            col_dot = '#7BC47F' if balances[i] >= 0 else '#E05555'
            dots += f'<circle cx="{x}" cy="{y}" r="3" fill="{col_dot}"/>'

        # month labels
        labels = ''
        for i, lbl in enumerate(month_labels):
            x, _ = pts[i]
            labels += f'<text x="{x}" y="{H+2}" text-anchor="middle" font-size="7" fill="{c("text3")}">{lbl}</text>'

        svg = f'''<svg viewBox="0 0 {W} {H+12}" xmlns="http://www.w3.org/2000/svg">
  <line x1="0" y1="{zero_y}" x2="{W}" y2="{zero_y}"
        stroke="{c("card_border")}" stroke-width="1" stroke-dasharray="3,3"/>
  <polyline points="{polyline}" fill="none"
            stroke="{c("teal")}" stroke-width="2" stroke-linejoin="round"/>
  {dots}
  {labels}
</svg>'''

        return ft.Container(
            ft.Column([
                txt(T['chart_cumul_balance'], size=11,
                    weight=ft.FontWeight.W_600, col='text3'),
                ft.Container(height=6),
                ft.Container(
                    ft.Image(src=f'data:image/svg+xml;base64,{_svg_b64(svg)}',
                             fit=ft.BoxFit.FIT_WIDTH,
                             expand=True),
                    expand=True,
                ),
                ft.Container(height=4),
                txt(f"{T['chart_final']}: {fmt(balances[-1])} {currency}",
                    size=12, weight=ft.FontWeight.W_700,
                    col='green' if balances[-1] >= 0 else 'danger'),
            ], spacing=0),
            padding=14, bgcolor=c('card'),
            border=B.all(1, c('card_border')), border_radius=12,
        )

    # ── Summary totals ────────────────────────────────────────────────────────
    def summary_row() -> ft.Row:
        total_rev = sum(revenues)
        total_exp = sum(expenses)
        net       = total_rev - total_exp

        def card(label, val, col_key):
            return ft.Container(
                ft.Column([
                    txt(label, size=9, weight=ft.FontWeight.W_600, col='text3'),
                    txt(f"{fmt(val)} {currency}", size=14,
                        weight=ft.FontWeight.W_700, col=col_key),
                ], spacing=2),
                expand=True, padding=12,
                bgcolor=c('card'), border=B.all(1, c('card_border')),
                border_radius=10,
            )

        return ft.Row([
            card(T['chart_total_income'],  total_rev, 'teal'),
            card(T['chart_total_exp'],     total_exp, 'danger'),
            card(T['chart_net'],           net,
                 'green' if net >= 0 else 'danger'),
        ], spacing=8)

    return ft.Column([
        txt(T['tab_charts'], size=20, weight=ft.FontWeight.W_700,
            family='Playfair Display', col='text'),
        summary_row(),
        bar_chart(),
        balance_chart(),
        ft.Container(height=40),
    ], spacing=12, scroll=ft.ScrollMode.AUTO, expand=True)


def _svg_b64(svg: str) -> str:
    import base64
    return base64.b64encode(svg.encode()).decode()