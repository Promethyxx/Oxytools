"""
Oxycash – Data model (mirrors the HTML data structure exactly)
"""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import List, Optional
import json, copy, datetime

MONTHS    = ['JAN','FEB','MAR','APR','MAI','JUN','JUL','AUG','SEP','OCT','NOV','DEC']
MNAMES    = ['Janvier','Février','Mars','Avril','Mai','Juin','Juillet','Août',
             'Septembre','Octobre','Novembre','Décembre']
SPECIAL_TABS = ['Dettes','Epargne','Frais','Viabilite','Config']
ALL_TABS  = MONTHS + SPECIAL_TABS


# ─── Primitives ───────────────────────────────────────────────────────────────

@dataclass
class Payment:
    date: str      # YYYY-MM-DD
    amount: float

    def to_dict(self):
        return {'date': self.date, 'amount': self.amount}

    @staticmethod
    def from_dict(d: dict) -> 'Payment':
        return Payment(date=d['date'], amount=float(d.get('amount', 0)))


@dataclass
class Line:
    name: str
    banque: float = 0.0
    cash: float = 0.0
    payments: List[Payment] = field(default_factory=list)
    # recurring: None or {'freq': int, 'start': 'JAN'} — freq in months
    recurring: Optional[dict] = None

    # ── computed ──
    def etat(self) -> float:
        return sum(p.amount for p in self.payments)

    def solde(self) -> float:
        return (self.banque + self.cash) - self.etat()

    def to_dict(self):
        d = {'name': self.name, 'banque': self.banque, 'cash': self.cash,
             'payments': [p.to_dict() for p in self.payments]}
        if self.recurring:
            d['recurring'] = self.recurring
        return d

    @staticmethod
    def from_dict(d: dict) -> 'Line':
        return Line(
            name=d.get('name', ''),
            banque=float(d.get('banque', 0)),
            cash=float(d.get('cash', 0)),
            payments=[Payment.from_dict(p) for p in d.get('payments', [])],
            recurring=d.get('recurring', None),
        )


def mk_line(name: str, banque: float = 0, cash: float = 0,
            payments: Optional[List[dict]] = None) -> Line:
    pays = [Payment.from_dict(p) for p in (payments or [])]
    return Line(name=name, banque=banque, cash=cash, payments=pays)


# ─── Month ────────────────────────────────────────────────────────────────────

@dataclass
class Month:
    name: str
    revenus:   List[Line] = field(default_factory=list)
    retraits:  List[Line] = field(default_factory=list)
    fixes:     List[Line] = field(default_factory=list)
    variables: List[Line] = field(default_factory=list)

    # section helpers
    def section(self, key: str) -> List[Line]:
        return getattr(self, key)

    def to_dict(self):
        return {
            'name': self.name,
            'revenus':   [l.to_dict() for l in self.revenus],
            'retraits':  [l.to_dict() for l in self.retraits],
            'fixes':     [l.to_dict() for l in self.fixes],
            'variables': [l.to_dict() for l in self.variables],
        }

    @staticmethod
    def from_dict(d: dict) -> 'Month':
        return Month(
            name=d.get('name', ''),
            revenus=[Line.from_dict(l) for l in d.get('revenus', [])],
            retraits=[Line.from_dict(l) for l in d.get('retraits', [])],
            fixes=[Line.from_dict(l) for l in d.get('fixes', [])],
            variables=[Line.from_dict(l) for l in d.get('variables', [])],
        )


# ─── Special sections ─────────────────────────────────────────────────────────

@dataclass
class Dette:
    rep: str = ''
    creancier: str = ''
    poursuite: str = ''
    solde: float = 0
    soldeOk: float = 0
    etat: str = ''
    date: str = ''

    def to_dict(self): return self.__dict__.copy()

    @staticmethod
    def from_dict(d): return Dette(**{k: d.get(k, v) for k, v in Dette().__dict__.items()})


@dataclass
class EpargneSondage:
    name: str = 'Nouveau'
    total: float = 0
    goal: float = 0
    def to_dict(self): return self.__dict__.copy()
    @staticmethod
    def from_dict(d): return EpargneSondage(**{k: d.get(k, 0) if k != 'name' else d.get(k, 'Nouveau') for k in ('name','total','goal')})


@dataclass
class EpargneAchat:
    name: str = 'Nouveau'
    prix: float = 0
    def to_dict(self): return self.__dict__.copy()
    @staticmethod
    def from_dict(d): return EpargneAchat(name=d.get('name','Nouveau'), prix=float(d.get('prix',0)))


@dataclass
class EpargnePcLegacy:
    name: str = 'Nouveau'
    val: float = 0
    def to_dict(self): return self.__dict__.copy()
    @staticmethod
    def from_dict(d): return EpargnePcLegacy(name=d.get('name','Nouveau'), val=float(d.get('val',0)))


@dataclass
class FraisLine:
    name: str = 'Nouveau'
    monthly: List[float] = field(default_factory=lambda: [0.0]*12)
    def to_dict(self): return {'name': self.name, 'monthly': self.monthly}
    @staticmethod
    def from_dict(d): return FraisLine(name=d.get('name',''), monthly=[float(x) for x in d.get('monthly',[0]*12)])


@dataclass
class ViabilitePalier:
    salaire: float = 3000
    loyer: float = 1412
    assurance: float = 444
    fraisMin: float = 500
    impotM: float = 0
    subside: float = 0
    def to_dict(self): return self.__dict__.copy()
    @staticmethod
    def from_dict(d): return ViabilitePalier(**{k: float(d.get(k,0)) for k in ViabilitePalier().__dict__})


# ─── Root data ────────────────────────────────────────────────────────────────

@dataclass
class AppData:
    months: dict[str, Month] = field(default_factory=dict)
    dettes: List[Dette] = field(default_factory=list)
    epargne: dict = field(default_factory=dict)
    frais: dict = field(default_factory=dict)
    viabilite: List[ViabilitePalier] = field(default_factory=list)

    def to_dict(self):
        ep = {
            'sondages': [x.to_dict() for x in self.epargne.get('sondages', [])],
            'achats':   [x.to_dict() for x in self.epargne.get('achats', [])],
            'pc_legacy':[x.to_dict() for x in self.epargne.get('pc_legacy', [])],
        }
        fr = {
            'fixes':     [x.to_dict() for x in self.frais.get('fixes', [])],
            'ponctuels': [x.to_dict() for x in self.frais.get('ponctuels', [])],
        }
        return {
            'months': {k: v.to_dict() for k, v in self.months.items()},
            'dettes': [d.to_dict() for d in self.dettes],
            'epargne': ep,
            'frais': fr,
            'viabilite': [v.to_dict() for v in self.viabilite],
        }

    @staticmethod
    def from_dict(d: dict) -> 'AppData':
        months = {k: Month.from_dict(v) for k, v in d.get('months', {}).items()}
        dettes = [Dette.from_dict(x) for x in d.get('dettes', [])]
        ep_raw = d.get('epargne', {})
        epargne = {
            'sondages':  [EpargneSondage.from_dict(x) for x in ep_raw.get('sondages', [])],
            'achats':    [EpargneAchat.from_dict(x)   for x in ep_raw.get('achats', [])],
            'pc_legacy': [EpargnePcLegacy.from_dict(x) for x in ep_raw.get('pc_legacy', [])],
        }
        fr_raw = d.get('frais', {})
        frais = {
            'fixes':     [FraisLine.from_dict(x) for x in fr_raw.get('fixes', [])],
            'ponctuels': [FraisLine.from_dict(x) for x in fr_raw.get('ponctuels', [])],
        }
        viabilite = [ViabilitePalier.from_dict(x) for x in d.get('viabilite', [])]
        return AppData(months=months, dettes=dettes, epargne=epargne, frais=frais, viabilite=viabilite)

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), indent=2, ensure_ascii=False)

    @staticmethod
    def from_json(s: str) -> 'AppData':
        return AppData.from_dict(json.loads(s))


# ─── Default data factory (mirrors defaultData() in JS) ───────────────────────

def _def_month(mi: int) -> Month:
    return Month(
        name=MNAMES[mi],
        revenus=[
            mk_line('Assurance'), mk_line('Autre'),
            mk_line('Banque', banque=0.07), mk_line('Cash'),
            mk_line('CSR', banque=2090), mk_line('Remboursement'), mk_line('Sondage'),
        ],
        retraits=[mk_line('Retraits', banque=380)],
        fixes=[
            mk_line('BCV', banque=3.5), mk_line('Courses', banque=460),
            mk_line('Loisirs', cash=380), mk_line('Loyer', banque=900),
            mk_line('Salt home', banque=39.95), mk_line('Salt mobile', banque=20.95),
        ],
        variables=[],
    )


def default_data() -> AppData:
    d = AppData()
    for i, m in enumerate(MONTHS):
        d.months[m] = _def_month(i)

    # ── JAN overrides ──
    jan = d.months['JAN']
    jan.revenus[2].banque = 1.2;  jan.revenus[2].payments = [Payment('2026-01-01', 1.2)]
    jan.revenus[3].cash   = 5;    jan.revenus[3].payments = [Payment('2026-01-02', 5)]
    jan.revenus[4].banque = 2090; jan.revenus[4].payments = [Payment('2026-01-03', 2090)]
    jan.revenus[5].banque = 40;   jan.revenus[5].payments = [Payment('2026-01-15', 40)]
    jan.retraits = [mk_line('Retraits', banque=360, payments=[{'date':'2026-01-05','amount':360}])]
    jan.fixes[0].payments = [Payment('2026-01-01', 3.5)]
    jan.fixes[1].payments = [Payment('2026-01-07',112.30), Payment('2026-01-14',98.45),
                              Payment('2026-01-21',156.20), Payment('2026-01-28',119.50)]
    jan.fixes[2].cash=360; jan.fixes[2].banque=0; jan.fixes[2].payments=[Payment('2026-01-05',360)]
    jan.fixes[3].payments = [Payment('2026-01-01', 900)]
    jan.fixes[4].payments = [Payment('2026-01-15', 39.95)]
    jan.fixes[5].payments = [Payment('2026-01-15', 20.95)]
    jan.variables = [
        mk_line('Epargne'),
        mk_line('Galaxus',   banque=211,   payments=[{'date':'2026-01-08','amount':211}]),
        mk_line('Equity',    banque=63.96, payments=[{'date':'2026-01-10','amount':63.96}]),
        mk_line('Jacques Kebab', cash=5,   payments=[{'date':'2026-01-12','amount':5}]),
        mk_line('Maman'),
        mk_line('Tam',       cash=50,      payments=[{'date':'2026-01-18','amount':20},
                                                     {'date':'2026-01-25','amount':30}]),
        mk_line('Claude',    banque=18.82, payments=[{'date':'2026-01-20','amount':18.82}]),
    ]

    # ── FEB overrides ──
    feb = d.months['FEB']
    feb.revenus[4].payments = [Payment('2026-02-03', 2090)]
    feb.revenus[2].banque   = 0.07; feb.revenus[2].payments = [Payment('2026-02-01', 0.07)]
    feb.retraits = [mk_line('Retraits', banque=400, payments=[{'date':'2026-02-02','amount':20}])]
    feb.fixes[1].payments = [Payment('2026-02-06',55.80), Payment('2026-02-13',57.80)]
    feb.fixes[4].payments = [Payment('2026-02-15', 39.95)]
    feb.fixes[5].banque=39.75; feb.fixes[5].payments=[Payment('2026-02-15',39.75)]
    feb.fixes[2].cash=380; feb.fixes[2].banque=0
    feb.variables = [
        mk_line('Epargne', banque=200),
        mk_line('Maman', cash=20, payments=[{'date':'2026-02-10','amount':20}]),
        mk_line('Serafe', banque=83.75),
    ]

    # ── Dettes ──
    d.dettes = [
        Dette('Dr Plumez','Dr Plumez','7887912',403.60,204.75,'ADB','13.07.2017'),
        Dette('EOS','Dr Jean-Marie Monney','8007430',413.10,120.60,'ADB','03.01.2017'),
        Dette('Intrum AG','CFF','9150645',2804.25,2000,'Opposition totale','03.05.2019'),
        Dette('OFCOM','Serafe','7437384',270.65,270.65,'ADB','31.11.2015'),
        Dette('OFCOM','Serafe','7726673',315.35,315.35,'ADB','20.06.2016'),
        Dette('EOS','Swisscom','—',641,297,'Poursuite','?'),
    ]

    # ── Frais ──
    d.frais = {
        'fixes': [
            FraisLine('BCV',      [3.5]*12),
            FraisLine('Courses',  [460]*12),
            FraisLine('Loisirs',  [380]*12),
            FraisLine('Loyer',    [900]*12),
            FraisLine('Salt H',   [39.9]*12),
            FraisLine('Salt M',   [20.95]*12),
        ],
        'ponctuels': [
            FraisLine('ECA',       [20.7]+[0]*11),
            FraisLine('Serafe',    [0,83.75,0,0,83.75,0,0,83.75,0,0,83.75,0]),
            FraisLine('RC',        [0]*11+[89.15]),
            FraisLine('Romande',   [0,0,225,0,0,225,0,0,225,0,0,225]),
            FraisLine('Swisscaution',[0]*11+[139]),
        ],
    }

    # ── Épargne ──
    d.epargne = {
        'sondages': [
            EpargneSondage('AttaPoll',2.67,3), EpargneSondage('Club Décideur',0,5),
            EpargneSondage('Espace Opinion',33.2,25), EpargneSondage('Freecash',8.29,20),
            EpargneSondage('LP',0,0), EpargneSondage('Mobrog',0,5),
            EpargneSondage('Mypuls',22.3,30), EpargneSondage("Pawns.app",0,5),
            EpargneSondage('Polittrends',24.66,15), EpargneSondage('Surveoo',0,20),
            EpargneSondage('SurveyPolice',0,0), EpargneSondage('TGM Panel',0,15),
            EpargneSondage('Votre opinion',12.7,10), EpargneSondage('Yougov',2,50),
            EpargneSondage('Yuzuni',0,5),
        ],
        'achats': [
            EpargneAchat('Corsair 7000D',255), EpargneAchat('Asus Pro Art X870E Creator wifi',394.9),
            EpargneAchat('AMD Ryzen 9 9950X 3d',589), EpargneAchat('Kingston Fury',826),
            EpargneAchat('SSD Samsung 9100 Pro',452),
        ],
        'pc_legacy': [
            EpargnePcLegacy('Alim Corsair HX750i',80), EpargnePcLegacy('Asus Strix GTX 1060',80),
            EpargnePcLegacy('AsRock X99 Extrem 6',120), EpargnePcLegacy('Corsair Graphite 780 T',45),
            EpargnePcLegacy('Intel Core I7 5820K',169), EpargnePcLegacy('Ram HyperX Fury',44.6),
            EpargnePcLegacy('Samsung Evo 840',35), EpargnePcLegacy('Samsung Evo 850',60),
        ],
    }

    # ── Viabilité ──
    viab = []
    for s in range(3000, 17000, 500):
        assurance = 520 if s <= 6000 else 495 if s <= 7000 else 444
        subside   = 331 if s <= 3500 else 0
        viab.append(ViabilitePalier(
            salaire=s, loyer=1412, assurance=assurance,
            fraisMin=500, impotM=round(s*0.125), subside=subside
        ))
    d.viabilite = viab

    return d


# ─── Recurring propagation ────────────────────────────────────────────────────

def apply_recurring(data) -> None:
    """
    For every variable line with recurring set, ensure it exists in the
    correct target months (no payments copied, just name+amounts).
    Runs in-place on AppData. Safe to call multiple times (idempotent).
    """
    for src_key, src_month in data.months.items():
        src_mi = MONTHS.index(src_key)
        for line in src_month.variables:
            rec = line.recurring
            if not rec:
                continue
            freq  = int(rec.get('freq', 3))
            start = rec.get('start', src_key)
            start_mi = MONTHS.index(start) if start in MONTHS else src_mi
            for step in range(1, 13):
                target_mi = (start_mi + step * freq) % 12
                if target_mi == src_mi:
                    break   # full cycle, stop
                target_key   = MONTHS[target_mi]
                target_month = data.months.get(target_key)
                if target_month is None:
                    continue
                existing = [l.name for l in target_month.variables]
                if line.name not in existing:
                    target_month.variables.append(Line(
                        name=line.name,
                        banque=line.banque,
                        cash=line.cash,
                        recurring={'freq': freq, 'start': start},
                    ))


# ─── Budget month detection ────────────────────────────────────────────────────

def detect_budget_month() -> str:
    now = datetime.date.today()
    mi  = now.month - 1          # 0-based
    if now.day < 25:
        mi = (mi - 1) % 12
    return MONTHS[mi]


# ─── Formatting helpers ────────────────────────────────────────────────────────

def fmt(n: float) -> str:
    s = f"{n:.2f}"
    if s.endswith('00'): return s[:-3]
    if s.endswith('0'):  return s[:-1]
    return s

def fmt_sign(n: float) -> str:
    return ('+' if n >= 0 else '') + fmt(n)

def today() -> str:
    return datetime.date.today().isoformat()