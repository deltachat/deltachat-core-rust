Authentication-Results: mx.infomaniak.com; dmarc=none (p=none dis=none) header.from=delta.blinzeln.de
From: authresadding-attacker@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
Authentication-Results: mx.infomaniak.com; spf=none smtp.mailfrom=delta.blinzeln.de
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
