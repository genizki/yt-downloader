import { createContext, useContext } from "react";

type Bundle = Record<string, string>;

const en: Bundle = {
  "app.title": "yt-dlp",

  "search.placeholder": "Start typing for search",
  "search.hero": "What do you want to download?",
  "search.recent": "Recent",
  "search.results_count_template": 'About {count} results for "{query}"',
  "search.results_time_template": "in {ms}s",
  "search.results_pending": "…",
  "search.empty_with_key":
    "No results yet — still searching, or the query returned nothing.",
  "search.empty_no_key":
    "Set your YouTube Data API key in Settings → API to enable search.",

  "topbar.new_search": "New search",
  "topbar.open_settings": "Open settings",

  "queue.toggle": "Toggle download queue",
  "queue.label": "Queue",
  "queue.heading": "Download Queue",

  "settings.title": "Settings",
  "settings.eyebrow": "Preferences",
  "settings.done": "Done",
  "settings.close": "Close settings",

  "settings.section.downloads": "Downloads",
  "settings.section.constraints": "Constraints",
  "settings.section.extras": "Extras",
  "settings.section.appearance": "Appearance",
  "settings.section.authentication": "Authentication",
  "settings.section.api": "API",

  "settings.format.label": "Format",
  "settings.format.hint":
    "Container used when saving the file. Audio-only formats disable video options.",

  "settings.video_quality.label": "Video quality",
  "settings.video_quality.hint": "Maximum resolution to fetch",
  "settings.video_quality.disabled":
    "Disabled — current format is audio-only",

  "settings.video_codec.label": "Video codec",
  "settings.video_codec.hint": "Preferred video codec",
  "settings.video_codec.disabled": "Disabled — current format is audio-only",

  "settings.audio_quality.label": "Audio quality",
  "settings.audio_quality.hint": "Bitrate target for audio",

  "settings.download_path.label": "Download path",
  "settings.download_path.hint": "Files will be saved here",
  "settings.download_path.browse": "Browse…",

  "settings.max_size.label": "Maximum file size",
  "settings.max_size.hint":
    "Skip downloads larger than this. Useful on metered connections.",

  "settings.protocol.label": "Streaming protocol",
  "settings.protocol.hint": "Transport used to fetch the media stream.",

  "settings.theme.label": "Theme",
  "settings.theme.hint": "Light, Dark, or follow system",

  "settings.language.label": "Language",
  "settings.language.hint": "Interface language",

  "settings.auth.intro":
    "Optional. Provide a YouTube/Google access token to access private, age-restricted, or members-only content.",
  "settings.auth.token.label": "YouTube / Google access token",
  "settings.auth.help_link": "How to get a token?",
  "settings.auth.clear": "Clear token",

  "settings.api.key.label": "YouTube Data API key",
  "settings.api.key.hint":
    "Required for search. Get one at console.cloud.google.com → YouTube Data API v3.",
  "settings.api.playlist.label": "Auto-download playlists",
  "settings.api.playlist.hint":
    "Start downloading playlist items immediately, skipping the list view.",

  "extras.embedThumbnail.label": "Embed thumbnail",
  "extras.embedThumbnail.hint":
    "Attach the video's thumbnail as cover art.",
  "extras.embedMetadata.label": "Embed metadata",
  "extras.embedMetadata.hint":
    "Write title, uploader, and description into the file.",
  "extras.embedChapters.label": "Embed chapters",
  "extras.embedChapters.hint": "Include chapter markers from the source.",
  "extras.embedSubtitles.label": "Embed subtitles",
  "extras.embedSubtitles.hint":
    "Mux available subtitle tracks into the container.",
  "extras.writeSubtitles.label": "Write subtitles to file",
  "extras.writeSubtitles.hint":
    "Save subtitles as a separate .srt/.vtt file.",
  "extras.skipPlaylists.label": "Skip playlists",
  "extras.skipPlaylists.hint":
    "Download single video only, ignore playlist URLs.",
  "extras.restrictNames.label": "Restrict filenames",
  "extras.restrictNames.hint": "ASCII only, no spaces or special characters.",

  "download.queued": "queued",
  "download.downloading": "downloading",
  "download.post_processing": "post-processing",
  "download.moving": "moving",
  "download.done": "done ✓",
  "download.failed": "failed",
};

const de: Bundle = {
  "app.title": "yt-dlp",

  "search.placeholder": "Tippe um zu suchen",
  "search.hero": "Was möchtest du herunterladen?",
  "search.recent": "Zuletzt",
  "search.results_count_template": 'Etwa {count} Ergebnisse für "{query}"',
  "search.results_time_template": "in {ms}s",
  "search.results_pending": "…",
  "search.empty_with_key":
    "Noch keine Ergebnisse — Suche läuft, oder die Anfrage lieferte nichts.",
  "search.empty_no_key":
    "Hinterlege deinen YouTube-Data-API-Schlüssel in Einstellungen → API.",

  "topbar.new_search": "Neue Suche",
  "topbar.open_settings": "Einstellungen öffnen",

  "queue.toggle": "Download-Warteschlange umschalten",
  "queue.label": "Warteschlange",
  "queue.heading": "Download-Warteschlange",

  "settings.title": "Einstellungen",
  "settings.eyebrow": "Einstellungen",
  "settings.done": "Fertig",
  "settings.close": "Einstellungen schließen",

  "settings.section.downloads": "Downloads",
  "settings.section.constraints": "Einschränkungen",
  "settings.section.extras": "Extras",
  "settings.section.appearance": "Darstellung",
  "settings.section.authentication": "Authentifizierung",
  "settings.section.api": "API",

  "settings.format.label": "Format",
  "settings.format.hint":
    "Container, in dem die Datei gespeichert wird. Reine Audioformate deaktivieren die Videooptionen.",

  "settings.video_quality.label": "Videoqualität",
  "settings.video_quality.hint": "Maximale Auflösung, die geladen werden soll",
  "settings.video_quality.disabled":
    "Deaktiviert — aktuelles Format ist reines Audio",

  "settings.video_codec.label": "Video-Codec",
  "settings.video_codec.hint": "Bevorzugter Video-Codec",
  "settings.video_codec.disabled":
    "Deaktiviert — aktuelles Format ist reines Audio",

  "settings.audio_quality.label": "Audioqualität",
  "settings.audio_quality.hint": "Ziel-Bitrate für Audio",

  "settings.download_path.label": "Download-Pfad",
  "settings.download_path.hint": "Dateien werden hier gespeichert",
  "settings.download_path.browse": "Durchsuchen…",

  "settings.max_size.label": "Maximale Dateigröße",
  "settings.max_size.hint":
    "Größere Downloads überspringen. Praktisch bei volumenbegrenzten Verbindungen.",

  "settings.protocol.label": "Streaming-Protokoll",
  "settings.protocol.hint": "Transport, über den der Medienstream geladen wird.",

  "settings.theme.label": "Erscheinungsbild",
  "settings.theme.hint": "Hell, Dunkel oder System folgen",

  "settings.language.label": "Sprache",
  "settings.language.hint": "Sprache der Oberfläche",

  "settings.auth.intro":
    "Optional. Hinterlege einen YouTube-/Google-Zugriffstoken, um auf private, altersbeschränkte oder mitgliederexklusive Inhalte zuzugreifen.",
  "settings.auth.token.label": "YouTube- / Google-Zugriffstoken",
  "settings.auth.help_link": "Wie bekomme ich ein Token?",
  "settings.auth.clear": "Token entfernen",

  "settings.api.key.label": "YouTube-Data-API-Schlüssel",
  "settings.api.key.hint":
    "Erforderlich für die Suche. Erhältlich auf console.cloud.google.com → YouTube Data API v3.",
  "settings.api.playlist.label": "Playlists automatisch laden",
  "settings.api.playlist.hint":
    "Lädt Playlist-Inhalte sofort herunter, ohne die Listenansicht zu zeigen.",

  "extras.embedThumbnail.label": "Vorschaubild einbetten",
  "extras.embedThumbnail.hint":
    "Bettet das Vorschaubild des Videos als Cover ein.",
  "extras.embedMetadata.label": "Metadaten einbetten",
  "extras.embedMetadata.hint":
    "Schreibt Titel, Uploader und Beschreibung in die Datei.",
  "extras.embedChapters.label": "Kapitel einbetten",
  "extras.embedChapters.hint": "Übernimmt Kapitelmarken aus der Quelle.",
  "extras.embedSubtitles.label": "Untertitel einbetten",
  "extras.embedSubtitles.hint":
    "Bindet verfügbare Untertitelspuren in den Container ein.",
  "extras.writeSubtitles.label": "Untertitel als Datei speichern",
  "extras.writeSubtitles.hint":
    "Speichert Untertitel als separate .srt-/.vtt-Datei.",
  "extras.skipPlaylists.label": "Playlists überspringen",
  "extras.skipPlaylists.hint":
    "Lädt nur das einzelne Video, ignoriert Playlist-URLs.",
  "extras.restrictNames.label": "Dateinamen einschränken",
  "extras.restrictNames.hint":
    "Nur ASCII-Zeichen, keine Leer- oder Sonderzeichen.",

  "download.queued": "wartet",
  "download.downloading": "lädt",
  "download.post_processing": "konvertiert",
  "download.moving": "verschiebt",
  "download.done": "fertig ✓",
  "download.failed": "fehlgeschlagen",
};

const fr: Bundle = {
  "app.title": "yt-dlp",

  "search.placeholder": "Commencez à taper pour rechercher",
  "search.hero": "Que voulez-vous télécharger ?",
  "search.recent": "Récent",
  "search.results_count_template": 'Environ {count} résultats pour "{query}"',
  "search.results_time_template": "en {ms}s",
  "search.results_pending": "…",
  "search.empty_with_key":
    "Aucun résultat — recherche en cours, ou la requête n'a rien retourné.",
  "search.empty_no_key":
    "Définissez votre clé API YouTube Data dans Paramètres → API.",

  "topbar.new_search": "Nouvelle recherche",
  "topbar.open_settings": "Ouvrir les paramètres",

  "queue.toggle": "Basculer la file de téléchargement",
  "queue.label": "File",
  "queue.heading": "File de téléchargement",

  "settings.title": "Paramètres",
  "settings.eyebrow": "Préférences",
  "settings.done": "Terminé",
  "settings.close": "Fermer les paramètres",

  "settings.section.downloads": "Téléchargements",
  "settings.section.constraints": "Contraintes",
  "settings.section.extras": "Extras",
  "settings.section.appearance": "Apparence",
  "settings.section.authentication": "Authentification",
  "settings.section.api": "API",

  "settings.format.label": "Format",
  "settings.format.hint":
    "Conteneur utilisé pour enregistrer le fichier. Les formats audio désactivent les options vidéo.",

  "settings.video_quality.label": "Qualité vidéo",
  "settings.video_quality.hint": "Résolution maximale à récupérer",
  "settings.video_quality.disabled":
    "Désactivé — le format actuel est uniquement audio",

  "settings.video_codec.label": "Codec vidéo",
  "settings.video_codec.hint": "Codec vidéo préféré",
  "settings.video_codec.disabled":
    "Désactivé — le format actuel est uniquement audio",

  "settings.audio_quality.label": "Qualité audio",
  "settings.audio_quality.hint": "Débit cible pour l'audio",

  "settings.download_path.label": "Chemin de téléchargement",
  "settings.download_path.hint": "Les fichiers seront enregistrés ici",
  "settings.download_path.browse": "Parcourir…",

  "settings.max_size.label": "Taille de fichier maximale",
  "settings.max_size.hint":
    "Ignorer les téléchargements plus grands. Utile sur connexions limitées.",

  "settings.protocol.label": "Protocole de streaming",
  "settings.protocol.hint": "Transport utilisé pour récupérer le flux média.",

  "settings.theme.label": "Thème",
  "settings.theme.hint": "Clair, sombre, ou suivre le système",

  "settings.language.label": "Langue",
  "settings.language.hint": "Langue de l'interface",

  "settings.auth.intro":
    "Optionnel. Fournissez un jeton d'accès YouTube/Google pour accéder au contenu privé, restreint ou réservé aux membres.",
  "settings.auth.token.label": "Jeton d'accès YouTube / Google",
  "settings.auth.help_link": "Comment obtenir un jeton ?",
  "settings.auth.clear": "Supprimer le jeton",

  "settings.api.key.label": "Clé API YouTube Data",
  "settings.api.key.hint":
    "Requise pour la recherche. À obtenir sur console.cloud.google.com → YouTube Data API v3.",
  "settings.api.playlist.label": "Télécharger les playlists automatiquement",
  "settings.api.playlist.hint":
    "Lance immédiatement le téléchargement des éléments, sans afficher la liste.",

  "extras.embedThumbnail.label": "Intégrer la miniature",
  "extras.embedThumbnail.hint":
    "Attache la miniature de la vidéo comme pochette.",
  "extras.embedMetadata.label": "Intégrer les métadonnées",
  "extras.embedMetadata.hint":
    "Écrit le titre, l'auteur et la description dans le fichier.",
  "extras.embedChapters.label": "Intégrer les chapitres",
  "extras.embedChapters.hint": "Inclut les marqueurs de chapitres de la source.",
  "extras.embedSubtitles.label": "Intégrer les sous-titres",
  "extras.embedSubtitles.hint":
    "Multiplexe les pistes de sous-titres disponibles dans le conteneur.",
  "extras.writeSubtitles.label": "Écrire les sous-titres dans un fichier",
  "extras.writeSubtitles.hint":
    "Enregistre les sous-titres dans un fichier .srt/.vtt séparé.",
  "extras.skipPlaylists.label": "Ignorer les playlists",
  "extras.skipPlaylists.hint":
    "Télécharger uniquement la vidéo, ignorer les URL de playlist.",
  "extras.restrictNames.label": "Restreindre les noms de fichiers",
  "extras.restrictNames.hint":
    "ASCII uniquement, sans espaces ni caractères spéciaux.",

  "download.queued": "en attente",
  "download.downloading": "téléchargement",
  "download.post_processing": "post-traitement",
  "download.moving": "déplacement",
  "download.done": "terminé ✓",
  "download.failed": "échec",
};

const ja: Bundle = {
  "app.title": "yt-dlp",

  "search.placeholder": "入力して検索",
  "search.hero": "何をダウンロードしますか？",
  "search.recent": "最近",
  "search.results_count_template": '"{query}" の検索結果 約 {count} 件',
  "search.results_time_template": "{ms}秒",
  "search.results_pending": "…",
  "search.empty_with_key":
    "結果がまだありません — 検索中、または該当なしです。",
  "search.empty_no_key":
    "検索を有効にするには、設定 → API で YouTube Data API キーを設定してください。",

  "topbar.new_search": "新しい検索",
  "topbar.open_settings": "設定を開く",

  "queue.toggle": "ダウンロードキューを切り替え",
  "queue.label": "キュー",
  "queue.heading": "ダウンロードキュー",

  "settings.title": "設定",
  "settings.eyebrow": "環境設定",
  "settings.done": "完了",
  "settings.close": "設定を閉じる",

  "settings.section.downloads": "ダウンロード",
  "settings.section.constraints": "制約",
  "settings.section.extras": "追加機能",
  "settings.section.appearance": "外観",
  "settings.section.authentication": "認証",
  "settings.section.api": "API",

  "settings.format.label": "形式",
  "settings.format.hint":
    "保存に使うコンテナ。音声のみの形式では映像オプションが無効になります。",

  "settings.video_quality.label": "映像品質",
  "settings.video_quality.hint": "取得する最大解像度",
  "settings.video_quality.disabled": "無効 — 現在の形式は音声のみです",

  "settings.video_codec.label": "映像コーデック",
  "settings.video_codec.hint": "優先する映像コーデック",
  "settings.video_codec.disabled": "無効 — 現在の形式は音声のみです",

  "settings.audio_quality.label": "音声品質",
  "settings.audio_quality.hint": "音声の目標ビットレート",

  "settings.download_path.label": "保存先",
  "settings.download_path.hint": "ファイルはここに保存されます",
  "settings.download_path.browse": "参照…",

  "settings.max_size.label": "最大ファイルサイズ",
  "settings.max_size.hint":
    "これより大きいダウンロードをスキップします。従量制回線に便利です。",

  "settings.protocol.label": "ストリーミングプロトコル",
  "settings.protocol.hint": "メディアストリーム取得に使う転送方式。",

  "settings.theme.label": "テーマ",
  "settings.theme.hint": "ライト、ダーク、またはシステムに従う",

  "settings.language.label": "言語",
  "settings.language.hint": "インターフェース言語",

  "settings.auth.intro":
    "任意。プライベート、年齢制限、メンバー限定のコンテンツにアクセスするには YouTube/Google アクセストークンを設定してください。",
  "settings.auth.token.label": "YouTube / Google アクセストークン",
  "settings.auth.help_link": "トークンの取得方法は？",
  "settings.auth.clear": "トークンを削除",

  "settings.api.key.label": "YouTube Data API キー",
  "settings.api.key.hint":
    "検索に必要です。console.cloud.google.com → YouTube Data API v3 で取得できます。",
  "settings.api.playlist.label": "プレイリストを自動ダウンロード",
  "settings.api.playlist.hint":
    "リストを表示せず、プレイリスト項目をすぐにダウンロードします。",

  "extras.embedThumbnail.label": "サムネイルを埋め込む",
  "extras.embedThumbnail.hint": "動画のサムネイルをカバーアートとして添付します。",
  "extras.embedMetadata.label": "メタデータを埋め込む",
  "extras.embedMetadata.hint":
    "タイトル、アップロード者、説明をファイルに書き込みます。",
  "extras.embedChapters.label": "チャプターを埋め込む",
  "extras.embedChapters.hint": "ソースのチャプターマーカーを含めます。",
  "extras.embedSubtitles.label": "字幕を埋め込む",
  "extras.embedSubtitles.hint":
    "利用可能な字幕トラックをコンテナにミックスします。",
  "extras.writeSubtitles.label": "字幕をファイルに書き出す",
  "extras.writeSubtitles.hint": "字幕を別の .srt/.vtt ファイルとして保存します。",
  "extras.skipPlaylists.label": "プレイリストをスキップ",
  "extras.skipPlaylists.hint":
    "単一の動画のみダウンロードし、プレイリスト URL を無視します。",
  "extras.restrictNames.label": "ファイル名を制限",
  "extras.restrictNames.hint": "ASCII のみ。スペースや特殊文字は使えません。",

  "download.queued": "待機中",
  "download.downloading": "ダウンロード中",
  "download.post_processing": "後処理中",
  "download.moving": "移動中",
  "download.done": "完了 ✓",
  "download.failed": "失敗",
};

const BUNDLES: Record<string, Bundle> = { en, de, fr, ja };

export type Lang = keyof typeof BUNDLES | string;

export const I18nContext = createContext<string>("en");

export function useLang(): string {
  return useContext(I18nContext);
}

export function translate(
  lang: string,
  key: string,
  vars?: Record<string, string | number>,
): string {
  const bundle = BUNDLES[lang] ?? BUNDLES.en;
  let s = bundle[key] ?? BUNDLES.en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
    }
  }
  return s;
}

export function useT() {
  const lang = useLang();
  return (key: string, vars?: Record<string, string | number>) =>
    translate(lang, key, vars);
}
