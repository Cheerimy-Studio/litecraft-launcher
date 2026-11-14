Imports System.Diagnostics
Imports System.IO

''' <summary>
''' LiteCraft 26.3 的路径、版本列表、核心文件与资源准备。
''' </summary>
Public Module ModLiteCraft

    ''' <summary>
    ''' 版本列表配置的 SHA-256。配置或镜像内容与此不符时视为被篡改，拒绝使用。
    ''' 更新 versions.json 后必须同步更新此值并重新发布启动器。
    ''' </summary>
    Private Const VersionsConfigSha256 As String = "78307d3f35fefa2382746bf3aac19e4544993c0a917b261dcbc36c73abc26a93"

    Private Function VersionsConfigUrls() As String()
        Const RepoPath As String = "Cheerimy-Studio/litecraft-launcher/main/manifests/versions.json"
        Return {
            "https://raw.githubusercontent.com/" & RepoPath,
            "https://ghproxy.net/https://raw.githubusercontent.com/" & RepoPath,
            "https://raw.gitmirror.com/" & RepoPath
        }
    End Function

    Private _VersionsConfig As JObject = Nothing
    Private ReadOnly _VersionsConfigLock As New Object()

    Public Function BaseDir() As String
        Return Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "LiteCraft")
    End Function

    Public Function BackendPath() As String
        Return Path.Combine(BaseDir(), "LiteCraft-Launther.exe")
    End Function

    Public Function GamePath() As String
        Return Path.Combine(BaseDir(), "litecraft.exe")
    End Function

    Public Function DataDir() As String
        Return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "LiteCraft")
    End Function

    Public Function ResourcesPath() As String
        Dim Bundled As String = Path.Combine(BaseDir(), "resources", "26.3")
        If File.Exists(Path.Combine(Bundled, "horizon-resources.json")) Then Return Bundled
        Return Path.Combine(DataDir(), "resources", "26.3")
    End Function

    Public Function GameDirPath() As String
        Return Path.Combine(DataDir(), "game")
    End Function

    Public Function HomePath() As String
        Return DataDir()
    End Function

    ''' <summary>
    ''' 游戏文件与资源是否已就绪。
    ''' </summary>
    Public Function Installed() As Boolean
        Return File.Exists(GamePath()) AndAlso File.Exists(Path.Combine(ResourcesPath(), "horizon-resources.json"))
    End Function

    Public Function IsLiteCraftVersion(Name As String) As Boolean
        Return Name IsNot Nothing AndAlso Name.StartsWith("LiteCraft ", StringComparison.OrdinalIgnoreCase)
    End Function

    ''' <summary>
    ''' 读取 GitHub 上的版本列表。按顺序尝试官方 raw 与镜像，每一份都必须通过内置 SHA-256 校验。
    ''' </summary>
    Public Function LoadVersionsConfig() As JObject
        SyncLock _VersionsConfigLock
            If _VersionsConfig IsNot Nothing Then Return _VersionsConfig
            Dim Errors As New List(Of String)
            For Each Url As String In VersionsConfigUrls()
                Try
                    Dim Text As String = NetRequestByClient(Url, Timeout:=15000)
                    Dim Actual As String = Sha256Text(Text)
                    If Not String.Equals(Actual, VersionsConfigSha256, StringComparison.OrdinalIgnoreCase) Then
                        Logger.Warn($"LiteCraft 版本列表校验失败（可能被劫持）：{Url}，实际 SHA-256 {Actual}")
                        Errors.Add(Url & "：SHA-256 校验失败")
                        Continue For
                    End If
                    Dim Config As JObject = JObject.Parse(Text)
                    If Config("versions") Is Nothing OrElse Config("versions").Type <> JTokenType.Array Then
                        Throw New Exception("配置缺少 versions 数组")
                    End If
                    _VersionsConfig = Config
                    Logger.Info("LiteCraft 版本列表来自 " & Url)
                    Return Config
                Catch ex As Exception
                    Logger.Warn(ex, "LiteCraft 版本列表加载失败：" & Url)
                    Errors.Add(Url & "：" & ex.Message)
                End Try
            Next
            Throw New Exception("无法获取 LiteCraft 版本列表，所有源均失败：" & vbCrLf & String.Join(vbCrLf, Errors))
        End SyncLock
    End Function

    Private Function Sha256Text(Text As String) As String
        Using Hasher As System.Security.Cryptography.SHA256 = System.Security.Cryptography.SHA256.Create()
            Return BitConverter.ToString(Hasher.ComputeHash(Encoding.UTF8.GetBytes(Text))).Replace("-", "").ToLowerInvariant()
        End Using
    End Function

    ''' <summary>
    ''' 下载页的版本列表。只包含 GitHub 配置里列出的 LiteCraft 版本。
    ''' </summary>
    Public LiteCraftVersionListLoader As New LoaderTask(Of String, DlClientListResult)("LiteCraft Version List", AddressOf LiteCraftVersionListMain)

    Private Sub LiteCraftVersionListMain(Loader As LoaderTask(Of String, DlClientListResult))
        '强制刷新时重新拉取配置
        SyncLock _VersionsConfigLock
            _VersionsConfig = Nothing
        End SyncLock
        Dim Config As JObject = LoadVersionsConfig()
        Dim Versions As New JArray()
        For Each Version As JObject In Config("versions")
            Dim Entry As New JObject()
            Entry("id") = Version("id").ToString()
            Entry("type") = "release"
            Entry("url") = Version("version_json").ToString()
            Entry("releaseTime") = Version("release_time").ToString()
            Entry("lore") = Version("name").ToString()
            Versions.Add(Entry)
        Next
        If Versions.Count = 0 Then Throw New Exception("LiteCraft 版本列表为空")
        Dim Value As New JObject()
        Value("versions") = Versions
        Loader.Output = New DlClientListResult With {
            .SourceName = "LiteCraft GitHub",
            .IsOfficial = False,
            .Value = Value
        }
    End Sub

    ''' <summary>
    ''' 等待核心文件处理的安装目录；为空表示当前安装不是 LiteCraft。
    ''' </summary>
    Public PendingInstallFolder As String = ""

    ''' <summary>
    ''' 原生安装完成后，下载并解压 LiteCraft 核心，再用 PCL 安装好的原版文件准备资源。
    ''' </summary>
    Public Sub AfterInstall(Folder As String)
        RunInNewThread(
            Sub()
                Try
                    Hint("正在获取 LiteCraft 核心文件…", HintType.Blue)
                    If Not DownloadCore() Then Throw New Exception("核心文件下载或校验失败，请检查网络后重试")

                    Dim InstanceName As String = New DirectoryInfo(Folder.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)).Name
                    Dim VersionJson As String = Path.Combine(Folder, InstanceName & ".json")
                    Dim ClientJar As String = Path.Combine(Folder, InstanceName & ".jar")
                    Dim ResDir As String = Path.Combine(BaseDir(), "resources", "26.3")

                    Hint("正在准备 LiteCraft 资源，这一步需要几分钟…", HintType.Blue)
                    Dim Code As Integer = Prepare(McFolderSelected, ResDir,
                        If(File.Exists(VersionJson), VersionJson, Nothing),
                        If(File.Exists(ClientJar), ClientJar, Nothing))
                    If Code <> 0 Then Throw New Exception("资源准备失败，退出码 " & Code & "，详细信息见日志")
                    If Not File.Exists(Path.Combine(ResDir, "horizon-resources.json")) Then
                        Throw New Exception("资源准备没有生成 horizon-resources.json")
                    End If

                    EnsureVersion()
                    Hint("LiteCraft 26.3 已安装完成，可以在启动页选择并启动", HintType.Green)
                Catch ex As Exception
                    Logger.Error(ex, "LiteCraft 安装后处理失败")
                    RunInUi(Sub() MyMsgBox("LiteCraft 核心文件或资源准备失败：" & vbCrLf & ex.Message, "LiteCraft 26.3", "确定", IsWarn:=True))
                End Try
            End Sub, "LiteCraft Install")
    End Sub

    Private Function DownloadCore() As Boolean
        Dim Config As JObject = LoadVersionsConfig()
        Dim Versions As JArray = TryCast(Config("versions"), JArray)
        If Versions Is Nothing OrElse Versions.Count = 0 Then Throw New Exception("版本配置为空")
        Dim Core As JObject = TryCast(Versions(0)("core"), JObject)
        If Core Is Nothing Then Throw New Exception("版本配置缺少 core")

        Dim Sha As String = Core("sha256").ToString().ToLowerInvariant()
        Dim Size As Long = If(Core("size") Is Nothing, 0L, Core("size").Value(Of Long)())
        Dim Urls As New List(Of String) From {Core("url").ToString()}
        If Core("mirrors") IsNot Nothing Then
            For Each Mirror As JToken In Core("mirrors")
                Urls.Add(Mirror.ToString())
            Next
        End If

        Dim Temp As String = Path.Combine(Path.GetTempPath(), "LiteCraft-Core-26.3-" & Guid.NewGuid().ToString("N") & ".zip")
        Try
            For Each Url As String In Urls
                Try
                    Logger.Info("下载 LiteCraft 核心文件：" & Url)
                    Using Client As New WebClient()
                        Client.Headers("User-Agent") = "LiteCraft-Launcher"
                        Client.DownloadFile(Url, Temp)
                    End Using
                    Dim Actual As String = CryptographyUtils.ComputeFileHash(Temp, CryptographyUtils.HashMethod.Sha256)
                    If Not String.Equals(Actual, Sha, StringComparison.OrdinalIgnoreCase) Then
                        Logger.Warn($"LiteCraft 核心文件校验失败：{Url}，实际 SHA-256 {Actual}")
                        File.Delete(Temp)
                        Continue For
                    End If
                    If Size > 0 AndAlso New FileInfo(Temp).Length <> Size Then
                        Logger.Warn($"LiteCraft 核心文件大小不符：{Url}")
                        File.Delete(Temp)
                        Continue For
                    End If
                    Directory.CreateDirectory(BaseDir())
                    Using Archive As IO.Compression.ZipArchive = IO.Compression.ZipFile.OpenRead(Temp)
                        For Each Entry As IO.Compression.ZipArchiveEntry In Archive.Entries
                            Dim Dest As String = Path.Combine(BaseDir(), Entry.FullName)
                            If String.IsNullOrEmpty(Entry.Name) Then
                                Directory.CreateDirectory(Dest)
                            Else
                                Directory.CreateDirectory(Path.GetDirectoryName(Dest))
                                Entry.ExtractToFile(Dest, True)
                            End If
                        Next
                    End Using
                    Logger.Info("LiteCraft 核心文件已解压到 " & BaseDir())
                    Return True
                Catch ex As Exception
                    Logger.Warn(ex, "LiteCraft 核心文件下载失败：" & Url)
                    Try
                        If File.Exists(Temp) Then File.Delete(Temp)
                    Catch
                    End Try
                End Try
            Next
        Finally
            Try
                If File.Exists(Temp) Then File.Delete(Temp)
            Catch
            End Try
        End Try
        Return False
    End Function

    ''' <summary>
    ''' 调用 Rust 后端下载并准备 26.3 资源。返回进程退出码。
    ''' </summary>
    Public Function Prepare(Optional minecraftDir As String = "", Optional resourcesDir As String = "",
                            Optional versionJson As String = Nothing, Optional clientJar As String = Nothing) As Integer
        If Not File.Exists(BackendPath()) Then
            Logger.Warn("LiteCraft 后端不存在：" & BackendPath())
            Return -1
        End If
        Try
            Dim Arguments As String = "--prepare --source official"
            If Not String.IsNullOrEmpty(minecraftDir) Then Arguments &= " --minecraft-dir " & Quote(minecraftDir)
            If Not String.IsNullOrEmpty(resourcesDir) Then Arguments &= " --resources-dir " & Quote(resourcesDir)
            If Not String.IsNullOrEmpty(versionJson) Then Arguments &= " --version-json " & Quote(versionJson)
            If Not String.IsNullOrEmpty(clientJar) Then Arguments &= " --client-jar " & Quote(clientJar)

            Dim Info As New ProcessStartInfo(BackendPath(), Arguments) With {
                .WorkingDirectory = BaseDir(),
                .UseShellExecute = False,
                .RedirectStandardOutput = True,
                .RedirectStandardError = True,
                .CreateNoWindow = True,
                .StandardOutputEncoding = Encoding.UTF8,
                .StandardErrorEncoding = Encoding.UTF8
            }
            Dim P As New Process With {.StartInfo = Info}
            AddHandler P.OutputDataReceived, Sub(sender, e) If e.Data IsNot Nothing Then Logger.Info("[LiteCraft] " & e.Data)
            AddHandler P.ErrorDataReceived, Sub(sender, e) If e.Data IsNot Nothing Then Logger.Info("[LiteCraft] " & e.Data)
            P.Start()
            P.BeginOutputReadLine()
            P.BeginErrorReadLine()
            P.WaitForExit()
            Return P.ExitCode
        Catch ex As Exception
            Logger.Warn(ex, "LiteCraft 资源准备失败")
            Return -1
        End Try
    End Function

    Private Function Quote(Value As String) As String
        Return Chr(34) & Value & Chr(34)
    End Function

    ''' <summary>
    ''' 把 LiteCraft 26.3 注册到 PCL 的版本列表，供启动页直接启动。
    ''' </summary>
    Public Sub EnsureVersion()
        Try
            If String.IsNullOrEmpty(McFolderSelected) Then Return
            If Not Installed() Then Return

            Dim VersionName As String = "LiteCraft 26.3"
            Dim Folder As String = Path.Combine(McFolderSelected, "versions", VersionName)
            Dim JsonPath As String = Path.Combine(Folder, VersionName & ".json")
            Dim MarkerPath As String = Path.Combine(Folder, "litecraft.json")
            Dim Marker As New JObject()
            Marker("game") = GamePath()
            Marker("resources") = ResourcesPath()
            Marker("gameDir") = GameDirPath()
            Marker("home") = HomePath()

            If File.Exists(JsonPath) AndAlso File.Exists(MarkerPath) Then
                Try
                    Dim Old As JObject = JObject.Parse(File.ReadAllText(MarkerPath))
                    If Old("game")?.ToString() = GamePath() AndAlso Old("resources")?.ToString() = ResourcesPath() Then Return
                Catch
                End Try
            End If

            Directory.CreateDirectory(Folder)
            Dim SourceJson As String = Nothing
            For Each Candidate As String In {
                Path.Combine(DataDir(), "minecraft", "versions", "26.3", "26.3.json"),
                Path.Combine(BaseDir(), "minecraft", "versions", "26.3", "26.3.json"),
                Path.Combine(BaseDir(), "version", "26.3.json"),
                Path.Combine(McFolderSelected, "versions", "26.3", "26.3.json")
            }
                If File.Exists(Candidate) Then
                    SourceJson = File.ReadAllText(Candidate)
                    Exit For
                End If
            Next

            Dim Root As JObject
            If SourceJson IsNot Nothing Then
                Root = JObject.Parse(SourceJson)
            Else
                Root = New JObject()
                Root("id") = "26.3"
                Root("type") = "release"
                Root("mainClass") = "net.minecraft.client.main.Main"
                Root("releaseTime") = "2026-09-15T11:23:02+00:00"
                Root("assets") = "34"
                Root("assetIndex") = New JObject From {New JProperty("id", "34")}
                Root("downloads") = New JObject()
                Root("libraries") = New JArray()
                Root("arguments") = New JObject From {New JProperty("game", New JArray()), New JProperty("jvm", New JArray())}
            End If
            Root("id") = VersionName
            Root("litecraft") = Marker

            File.WriteAllText(JsonPath, Root.ToString())
            File.WriteAllText(MarkerPath, Marker.ToString())
            Try
                LoaderFolderRun(McInstanceListLoader, McFolderSelected, LoaderFolderRunType.ForceRun, MaxDepth:=1, ExtraPath:="versions\")
            Catch
            End Try
            Logger.Info("LiteCraft 26.3 已添加到版本列表")
        Catch ex As Exception
            Logger.Warn(ex, "添加 LiteCraft 版本失败")
        End Try
    End Sub

End Module
