static PROVIDERS_XML: &str = r#"
<root>

    <provider name="Aktivix" page="/aktivix-org" needsPreperation="true">
        <domains>
            <domain>aktivix.org</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2018-10"
        />
        <content>
            <h2 id="coments">Coments</h2>
<ul>
  <li>new Registrations are Invite only</li>
</ul>

<h2 id="preparations">Preparations</h2>
<p>Settings in Deltachat(Account setup):</p>
<div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>IMAP newyear.aktivix.org:143
SMTP newyear.aktivix.org:25
sec: STARTTLS
</code></pre></div></div>

        </content>

    </provider>

    <provider name="All-Inkl.com" page="/all-inkl-com" needsPreperation="false">
        <domains>
            <domain>all-inkl.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2018-08"
        />
        <content>
            

        </content>

    </provider>

    <provider name="Aol. (America Online)" page="/aol" needsPreperation="true">
        <domains>
            <domain>aol.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-12"
        />
        <content>
            <h2 id="preparations">Preparations</h2>

<p>send one mail via web and solve the captcha</p>

        </content>

    </provider>

    <provider name="Autistici/Inventati" page="/autistici-org" needsPreperation="false">
        <domains>
            <domain>autistici.org</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2018-01"
        />
        <content>
            

        </content>

    </provider>

    <provider name="Bitmessage Mail Gateway" page="/bitmessage-ch" needsPreperation="true">
        <domains>
            <domain>bitmessage.ch</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2018-10"
        />
        <content>
            <h2 id="comments">Comments</h2>
<ul>
  <li>Registration of new Accounts is closed (at time of writing 24. April 2019)</li>
</ul>

<h2 id="preparations">Preparations</h2>

<p>DeltaChat Account Settings:</p>
<div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>IMAP mail.bitmessage.ch:993 SSL/TLS
SMTP mail.bitmessage.ch:465 SSL/TLS
</code></pre></div></div>

        </content>

    </provider>

    <provider name="Bluewin" page="/bluewin-ch" needsPreperation="true">
        <domains>
            <domain>bluewin.ch</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="boolean"
            date="YYYY-MM"
        />
        <content>
            <h2 id="comments">Comments</h2>
<ul>
  <li><strong>geo locked</strong> to switzerland</li>
</ul>

<h2 id="preparations-to-get-account">Preparations to get account</h2>
<ul>
  <li>activation code delivered to physical mailing address, or swisscom mobile number</li>
</ul>

        </content>

    </provider>

    <provider name="cock.li" page="/cock-li" needsPreperation="true">
        <domains>
            <domain>cock.li</domain>
        
            <domain>(26 other provocative domains)</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2019/07"
        />
        <content>
            <h2 id="comments">Comments</h2>
<p>Free/Hate Speech (due to the domains not people in my experience) mail service used by some people in the Fediverse that provides some domains: cock.li, airmail.cc, 8chan.co, redchan.it, 420blaze.it, aaathats3as.com, cumallover.me, dicksinhisan.us, loves.dicksinhisan.us, wants.dicksinhisan.us, dicksinmyan.us, loves.dicksinmyan.us, wants.dicksinmyan.us, goat.si, horsefucker.org, national.shitposting.agency, nigge.rs, tfwno.gf, cock.lu, cock.email, firemail.cc, hitler.rocks, getbackinthe.kitchen, memeware.net, cocaine.ninja, waifu.club, rape.lol and nuke.africa.
It seems has no policy about sending many mails https://cock.li/tos: A user reported to be banned some days after using it. No problem with other users.</p>

<h2 id="preparations">Preparations</h2>
<p>mail.cock.li both for IMAP and SMTP</p>

        </content>

    </provider>

    <provider name="Comcast / xfinity" page="/comcast" needsPreperation="false">
        <domains>
            <domain>xfinity.com</domain>
        
            <domain>comcast.net</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-12"
        />
        <content>
            

        </content>

    </provider>

    <provider name="Disroot" page="/disroot" needsPreperation="false">
        <domains>
            <domain>disroot.org</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-06"
        />
        <content>
            

        </content>

    </provider>

    <provider name="FastMail" page="/fastmail-com" needsPreperation="true">
        <domains>
            <domain>fastmail.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailAppPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2018-09"
        />
        <content>
            <h2 id="preparations">Preparations</h2>

<p>Create an app-specific Password</p>

        </content>

    </provider>

    <provider name="freenet.de" page="/freenet-de" needsPreperation="false">
        <domains>
            <domain>freenet.de</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-06"
        />
        <content>
            

        </content>

    </provider>

    <provider name="Google Mail" page="/gmail" needsPreperation="true">
        <domains>
            <domain>gmail.com</domain>
        
            <domain>googlemail.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
         
            <auth>emailAppPass</auth>
         
            <auth>Oauth</auth>
        </auth-methods>
        <status
            works="true"
            date="2018-05"
        />
        <content>
            <h2 id="comments">Comments</h2>

<ul>
  <li>Additional information to the email sending/recieving limits can be found on https://support.google.com/mail/answer/22839?hl=en</li>
</ul>

<h2 id="preparations">Preparations</h2>

<h3 id="use-oauth-recomended">Use OAuth (recomended)</h3>

<p>When Deltachat asks you to use Oauth, accept and login in the google login that pops up.</p>

<h3 id="without-2fa">Without 2FA</h3>

<p>Enable “less-secure-apps” to allow non google programms to connect to your email account. (It is recomended to use Outh instead)</p>

<h3 id="with-2fa">With 2FA</h3>

<p>Create an “App Specific Passwort” for DeltaChat.</p>

        </content>

    </provider>

    <provider name="GMX.net" page="/gmx-net" needsPreperation="true">
        <domains>
            <domain>gmx.net</domain>
        
            <domain>gmx.de</domain>
        
            <domain>gmx.at</domain>
        
            <domain>gmx.ch</domain>
        
            <domain>gmx.org (€)</domain>
        
            <domain>gmx.eu (€)</domain>
        
            <domain>gmx.info (€)</domain>
        
            <domain>gmx.biz (€)</domain>
        
            <domain>gmx.com (€)</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-06"
        />
        <content>
            <h2 id="preparations">Preparations</h2>
<p>activate access for external mail client at gmx side</p>

        </content>

    </provider>

    <provider name="hotmail / outlook / office365" page="/hotmail" needsPreperation="false">
        <domains>
            <domain>hotmail.com</domain>
        
            <domain>outlook.com</domain>
        
            <domain>office365.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="false"
            date="2019-02"
        />
        <content>
            <blockquote>
  <p>Detected to be not working any more, for details please check this <a href="https://github.com/deltachat/deltachat-core/issues/561">issue</a></p>
</blockquote>

<h2 id="delta">Delta</h2>
<div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>IMAP-Server: outlook.office365.com
Note: SMTP-Server can be omitted as outlook.office365.com is used (automatically by Delta Chat) and working. Other SMTP-Server that are working are smtp.office365.com 1 and smtp-mail.outlook.com 6.
</code></pre></div></div>

        </content>

    </provider>

    <provider name="I.ua" page="/i-ua" needsPreperation="false">
        <domains>
            <domain>i.ua</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-12"
        />
        <content>
            

        </content>

    </provider>

    <provider name="iCloud Mail" page="/icloud" needsPreperation="true">
        <domains>
            <domain>icloud.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailAppPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-05"
        />
        <content>
            <h2 id="preparations">Preparations</h2>

<p>For iCloud, you can’t use the regular password anymore, you have to <a href="https://support.apple.com/en-gb/HT204397">generate one for each external client</a>. Follow the <a href="https://support.apple.com/en-us/HT202304">generation instructions</a> and use that password in Delta Chat.</p>

        </content>

    </provider>

    <provider name="KONTENT" page="/kontent-com" needsPreperation="false">
        <domains>
            <domain>kontent.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2018-06"
        />
        <content>
            <h2 id="comments">Comments</h2>
<ul>
  <li>disable greylisting will avoid 5 min delay</li>
</ul>

        </content>

    </provider>

    <provider name="Mail.ru" page="/mail-ru" needsPreperation="false">
        <domains>
            <domain>mail.ru</domain>
        
            <domain>inbox.ru</domain>
        
            <domain>bk.ru</domain>
        
            <domain>list.ru</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2019-01"
        />
        <content>
            <h2 id="preparations">Preparations</h2>

<h3 id="deltachat-account-settings">DeltaChat Account Settings</h3>
<p>should be detected automaticaly, but here there are if they don’t get detected.</p>
<div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>imap.mail.ru port 993 SSL/TLS
smtp.mail.ru 587 SSL/TLS 
</code></pre></div></div>

        </content>

    </provider>

    <provider name="Mailbox.org" page="/mailbox-org" needsPreperation="false">
        <domains>
            <domain>mailbox.org</domain>
        
            <domain>secure.mailbox.org</domain>
        
            <domain>(custom)</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2019-03"
        />
        <content>
            <h2 id="comments">Comments:</h2>
<ul>
  <li>If you should reach some limits contact their support and they solve it for you <a href="https://userforum.mailbox.org/topic/the-limits-for-your-account-are-exceeded#comment-14091">source</a></li>
</ul>

<h2 id="using-with-custom-domain">Using with Custom Domain</h2>

<h3 id="deltachat-account-settings">DeltaChat Account settings</h3>
<div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>mailaddress - your extensions address
SMTP and IMAP login - main adress (not extentions address)
Server automated
Credentials: main address+password
</code></pre></div></div>

<h3 id="seperate-mail-from-chat">Seperate Mail from Chat</h3>
<p><a href="http://blog.lenzg.net/2019/02/using-delta-chat-with-email-sub-addresses/">Set Sieve rules for separate mail from chat - Tutorial</a></p>

        </content>

    </provider>

    <provider name="nauta.cu" page="/nauta-cu" needsPreperation="false">
        <domains>
            <domain>nauta.cu</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2019-04"
        />
        <content>
            <h2 id="comments">Comments</h2>
<ul>
  <li>Nauta email service provided by ETECSA in Cuba, accessible on mobile data connection or WiFi</li>
</ul>

        </content>

    </provider>

    <provider name="POSTEO" page="/posteo" needsPreperation="false">
        <domains>
            <domain>posteo.de</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-06"
        />
        <content>
            

        </content>

    </provider>

    <provider name="ProtonMail" page="/protonmail" needsPreperation="true">
        <domains>
            <domain>protonmail.com</domain>
        
            <domain>protonmail.ch</domain>
        </domains>
        <auth-methods> 
            <auth>emailAppPass</auth>
        </auth-methods>
        <status
            works="false"
            date="2019-02"
        />
        <content>
            <h2 id="preparation">Preparation</h2>
<p>ProtonMail requires a <a href="https://protonmail.com/bridge/">special Bridge</a> and a <strong>paid</strong> plan to use with DeltaChat.</p>

<p>But currently there is a Problem with that Bridge-Program:</p>

<p>See https://github.com/deltachat/deltachat-desktop/issues/669</p>


        </content>

    </provider>

    <provider name="riseup.net" page="/riseup-net" needsPreperation="false">
        <domains>
            <domain>riseup.net</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-12"
        />
        <content>
            <h2 id="comments">Comments</h2>
<ul>
  <li>new Registrations are Invite only</li>
</ul>

<blockquote>
  <p>Compared with Disroot, GMail and Hotmail. I got less problems and a good performance with accounts there. 
–<a href="https://support.delta.chat/u/echedeylr/">EchedeyLR</a> on the Forums</p>
</blockquote>

        </content>

    </provider>

    <provider name="rogers.com" page="/rogers-com" needsPreperation="true">
        <domains>
            <domain>rogers.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-12"
        />
        <content>
            

        </content>

    </provider>

    <provider name="strato.de (custom domain)" page="/strato-de" needsPreperation="true">
        <domains>
            <domain>(custom)</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2018-10"
        />
        <content>
            <h2 id="preparations">Preparations</h2>
<p>DeltaChat Account Settings:</p>
<div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>IMAP imap.strato.de:993 SSL/TLS
SMTP smtp.strato.de:465 SSL/TLS
Credentials: email+password
</code></pre></div></div>

        </content>

    </provider>

    <provider name="T-Online" page="/t-online" needsPreperation="true">
        <domains>
            <domain>t-online.de</domain>
        </domains>
        <auth-methods> 
            <auth>emailAppPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2019-03"
        />
        <content>
            <h2 id="preparations">Preparations</h2>
<p>Enter additional password for external access in web interface and use that to login.</p>

<blockquote>
  <p>Without setting an additional password, external account access is not possible</p>
</blockquote>

        </content>

    </provider>

    <provider name="UKR.NET" page="/ukr-net" needsPreperation="false">
        <domains>
            <domain>ukr.net</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-12"
        />
        <content>
            

        </content>

    </provider>

    <provider name="Verizon" page="/verizon" needsPreperation="false">
        <domains>
            <domain>verizon.net</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-12"
        />
        <content>
            

        </content>

    </provider>

    <provider name="VFEmail" page="/vfemail" needsPreperation="false">
        <domains>
            <domain>vfemail.net</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-10"
        />
        <content>
            

        </content>

    </provider>

    <provider name="web.de" page="/web-de" needsPreperation="true">
        <domains>
            <domain>web.de</domain>
        
            <domain>email.de (€)</domain>
        
            <domain>flirt.ms (€)</domain>
        
            <domain>hallo.ms (€)</domain>
        
            <domain>kuss.ms (€)</domain>
        
            <domain>love.ms (€)</domain>
        
            <domain>magic.ms (€)</domain>
        
            <domain>singles.ms (€)</domain>
        
            <domain>cool.ms (€)</domain>
        
            <domain>kanzler.ms (€)</domain>
        
            <domain>okay.ms (€)</domain>
        
            <domain>party.ms (€)</domain>
        
            <domain>pop.ms (€)</domain>
        
            <domain>stars.ms (€)</domain>
        
            <domain>techno.ms (€)</domain>
        
            <domain>clever.ms (€)</domain>
        
            <domain>deutschland.ms (€)</domain>
        
            <domain>genial.ms (€)</domain>
        
            <domain>ich.ms (€)</domain>
        
            <domain>online.ms (€)</domain>
        
            <domain>smart.ms (€)</domain>
        
            <domain>wichtig.ms (€)</domain>
        
            <domain>action.ms (€)</domain>
        
            <domain>fussball.ms (€)</domain>
        
            <domain>joker.ms (€)</domain>
        
            <domain>planet.ms (€)</domain>
        
            <domain>power.ms (€)</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-06"
        />
        <content>
            <h2 id="preparations">Preparations</h2>
<ol>
  <li>Allow to fetch mail via IMAP: In the Web UI, select E-Mail -&gt; Settings -&gt; E-Mail: POP3/IMAP -&gt; Allow POP3/IMAP access.</li>
  <li>make sure, relevant information are not sorted out by web.de 15 eg. to “unknown senders” and configure the web.de-inbox accordingly.</li>
</ol>

        </content>

    </provider>

    <provider name="yahoo" page="/yahoo" needsPreperation="true">
        <domains>
            <domain>yahoo.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-12"
        />
        <content>
            <h2 id="preparations">Preparations</h2>
<p>enable “less secure” apps</p>

        </content>

    </provider>

    <provider name="yandex.ru" page="/yandex-ru" needsPreperation="false">
        <domains>
            <domain>yandex.ru</domain>
        
            <domain>yandex.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2018-08"
        />
        <content>
            

        </content>

    </provider>

    <provider name="ziggo" page="/ziggo-nl" needsPreperation="false">
        <domains>
            <domain>ziggo.nl</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-06"
        />
        <content>
            

        </content>

    </provider>

    <provider name="ZOHO" page="/zoho-com" needsPreperation="false">
        <domains>
            <domain>zoho.com</domain>
        </domains>
        <auth-methods> 
            <auth>emailPass</auth>
        </auth-methods>
        <status
            works="true"
            date="2017-09"
        />
        <content>
            

        </content>

    </provider>

</root>"#;
