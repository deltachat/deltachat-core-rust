ARC-Authentication-Results: i=2; mx.microsoft.com 1; spf=pass (sender ip is
 40.92.73.85) smtp.rcpttodomain=outlook.com smtp.mailfrom=hotmail.com;
 dmarc=pass (p=none sp=none pct=100) action=none header.from=hotmail.com;
 dkim=pass (signature was verified) header.d=hotmail.com; arc=pass (0 oda=0
 ltdi=1)
Authentication-Results: spf=pass (sender IP is 40.92.73.85)
 smtp.mailfrom=hotmail.com; dkim=pass (signature was verified)
 header.d=hotmail.com;dmarc=pass action=none
 header.from=hotmail.com;compauth=pass reason=100
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@outlook.com>
