export classSearch Service {
   search(q?: string): string {
    return (q || '').trim().replace(/[%_\\]/g,'\\$');
  }
}
}
